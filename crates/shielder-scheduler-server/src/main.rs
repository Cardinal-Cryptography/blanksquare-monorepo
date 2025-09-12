mod aws_session_tokens;
mod command_line_args;
mod db;
mod error;
mod handlers;
mod relayer_rpc_controller;
mod scheduler_processor;

use std::{net::SocketAddrV4, sync::Arc, time::Duration};

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    serve, Router,
};
use clap::Parser;
use error::SchedulerServerError as Error;
use metrics_exporter_prometheus::PrometheusBuilder;
use shielder_scheduler_common::metrics::FutureHistogramLayer;
use tokio::{net::TcpListener, sync::Mutex, sync::watch};
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::{
    aws_session_tokens::AwsCredentials,
    command_line_args::CommandLineArgs,
    handlers::{self as server_handlers},
    relayer_rpc_controller::RelayerRpcController,
    scheduler_processor::SchedulerProcessor,
};

#[derive(Debug)]
struct AppState {
    options: CommandLineArgs,
    db_pool: db::PgPool,
    tee_task_pool: Arc<tokio_task_pool::Pool>,
    aws_credentials: Arc<Mutex<AwsCredentials>>,
    relayer_rpc_controller: RelayerRpcController,
    shutdown_tx: watch::Sender<bool>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let options = CommandLineArgs::parse();

    if let Err(validation_error) = options.validate() {
        return Err(Error::ParseError(validation_error));
    }

    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(EnvFilter::from_default_env()))
        // Initialize metrics collection
        .with(FutureHistogramLayer::with_all_spans().with_filter(EnvFilter::new("info")))
        .init();

    PrometheusBuilder::new()
        .with_http_listener(SocketAddrV4::new(
            options
                .bind_address
                .parse()
                .map_err(|_| Error::ParseError("Invalid bind address".to_string()))?,
            options.metrics_port,
        ))
        .set_bucket_duration(Duration::from_secs(options.metrics_bucket_duration_secs))?
        .upkeep_timeout(Duration::from_secs(options.metrics_upkeep_timeout_secs))
        .install()?;

    let db_pool = db::connect_to_db(&options).await?;
    db::create_tables(&db_pool).await?;

    let tee_task_pool = tokio_task_pool::Pool::bounded(options.tee_task_pool_capacity)
        .with_spawn_timeout(Duration::from_secs(options.tee_task_pool_timeout_secs))
        .with_run_timeout(Duration::from_secs(options.tee_compute_timeout_secs))
        .into();

    let aws_credentials = if !options.disable_kms {
        if let (Some(region), Some(iam_role)) = (&options.aws_region, &options.aws_iam_kms_role) {
            aws_session_tokens::get_session_token(
                region,
                options.aws_sts_refresh_period_secs as i32,
                iam_role,
            )
            .await?
        } else {
            return Err(Error::ParseError(
                "AWS_REGION and AWS_IAM_KMS_ROLE are required when --disable-kms is not set".into(),
            ));
        }
    } else {
        // Use dummy credentials for local development
        use crate::aws_session_tokens::AwsCredentials;
        AwsCredentials {
            access_key_id: "dummy-access-key".to_string(),
            secret_access_key: "dummy-secret-key".to_string(),
            session_token: Some("dummy-session-token".to_string()),
        }
    };

    let relayer_rpc_controller = RelayerRpcController::new(options.relayer_rpc_url.clone());
    
    // Create shutdown signal channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    
    let app_state = Arc::new(AppState {
        options,
        tee_task_pool,
        db_pool,
        aws_credentials: Arc::new(Mutex::new(aws_credentials)),
        relayer_rpc_controller,
        shutdown_tx,
    });

    // Perform initial TEE public key verification to ensure the server is correctly configured
    info!("Performing initial TEE public key verification...");
    let _verification_result =
        server_handlers::tee_public_key::tee_public_key(axum::extract::State(app_state.clone()))
            .await
            .map_err(|e| Error::ParseError(format!("TEE public key verification failed: {}", e)))?;

    info!("TEE public key verification successful");

    // Start the AWS credentials refresh task
    if !app_state.options.disable_kms {
        let credentials_refresh_state = app_state.clone();
        tokio::spawn(async move {
            aws_credentials_refresh_task(credentials_refresh_state).await;
        });
    }

    let scheduler_processor = SchedulerProcessor::new(app_state.clone());
    tokio::spawn(async move {
        scheduler_processor.start().await;
    });

    let listener = TcpListener::bind((
        app_state.options.bind_address.clone(),
        app_state.options.public_port,
    ))
    .await?;

    let router = Router::new()
        .route("/health", get(server_handlers::health::health))
        .route(
            "/public_key",
            get(server_handlers::tee_public_key::tee_public_key),
        )
        .route(
            "/schedule_withdraw",
            post(server_handlers::schedule_withdraw::schedule_withdraw),
        )
        .layer(DefaultBodyLimit::max(
            app_state.options.maximum_request_size,
        ))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    info!("Starting local server on {}", listener.local_addr()?);
    
    // Use graceful shutdown with signal handling
    let graceful = serve(listener, router).with_graceful_shutdown(async move {
        tokio::select! {
            // Wait for shutdown signal from credential refresh failure
            _ = async {
                let mut rx = shutdown_rx;
                let _ = rx.changed().await;
                if *rx.borrow() {
                    info!("Received shutdown signal due to AWS credential refresh failure");
                }
            } => {},
            // Wait for Ctrl+C
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C signal, starting graceful shutdown...");
            },
            // Wait for SIGTERM on Unix systems
            _ = async {
                #[cfg(unix)]
                {
                    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("Failed to register SIGTERM handler");
                    sigterm.recv().await;
                    info!("Received SIGTERM signal, starting graceful shutdown...");
                }
                #[cfg(not(unix))]
                {
                    // On non-Unix systems, just wait forever (this branch won't be taken due to tokio::select!)
                    std::future::pending::<()>().await;
                }
            } => {},
        }
    });
    
    graceful.await?;

    Ok(())
}

/// Background task that periodically refreshes AWS STS credentials
async fn aws_credentials_refresh_task(app_state: Arc<AppState>) {
    if !app_state.options.disable_kms {
        if let (Some(region), Some(iam_role)) = (&app_state.options.aws_region, &app_state.options.aws_iam_kms_role) {
            let mut interval = tokio::time::interval(Duration::from_secs(
                app_state.options.aws_sts_refresh_period_secs,
            ));

            // Skip the first tick since we already have initial credentials
            interval.tick().await;

            loop {
                interval.tick().await;

                info!("Refreshing AWS credentials from EC2 metadata");
                match aws_session_tokens::get_session_token(
                    region,
                    app_state.options.aws_sts_refresh_period_secs as i32,
                    iam_role,
                )
                .await
                {
                    Ok(new_credentials) => {
                        let mut credentials = app_state.aws_credentials.lock().await;
                        *credentials = new_credentials;
                        info!("AWS credentials refreshed successfully from EC2 metadata");
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to refresh AWS credentials from EC2 metadata: {:?}",
                            e
                        );
                        // Signal graceful shutdown on credential refresh failure
                        info!("Signaling server shutdown due to AWS credential refresh failure");
                        let _ = app_state.shutdown_tx.send(true);
                        return;
                    }
                }
            }
        } else {
            info!("AWS credentials refresh disabled (no AWS settings provided)");
            // Just keep the task alive but do nothing
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        }
    } else {
        info!("AWS credentials refresh disabled (--disable-kms flag is set)");
        // Just keep the task alive but do nothing
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    }
}
