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
pub use handlers::tee_prepare_relay_calldata::prepare_relay_calldata;
use metrics_exporter_prometheus::PrometheusBuilder;
use shielder_scheduler_common::metrics::FutureHistogramLayer;
use tokio::{net::TcpListener, sync::Mutex};
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
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Parse command line arguments
    let options = CommandLineArgs::parse();

    // Validate command line arguments
    if let Err(validation_error) = options.validate() {
        return Err(Error::ParseError(validation_error));
    }

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(EnvFilter::from_default_env()))
        // Initialize metrics collection
        .with(FutureHistogramLayer::with_all_spans().with_filter(EnvFilter::new("info")))
        .init();

    // Initialize Prometheus metrics
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

    // Connect to the database
    let db_pool = db::connect_to_db(&options).await?;

    // Initialize database tables
    db::create_tables(&db_pool).await?;

    // Initialize the TEE task pool
    let tee_task_pool = tokio_task_pool::Pool::bounded(options.tee_task_pool_capacity)
        .with_spawn_timeout(Duration::from_secs(options.tee_task_pool_timeout_secs))
        .with_run_timeout(Duration::from_secs(options.tee_compute_timeout_secs))
        .into();

    // Get AWS credentials from EC2 instance metadata
    #[cfg(not(feature = "local-run"))]
    let aws_credentials = aws_session_tokens::get_session_token(
        &options.aws_region,
        options.aws_sts_refresh_period_secs as i32,
        &options.aws_iam_kms_role,
    )
    .await?;

    #[cfg(feature = "local-run")]
    let aws_credentials = if let (Some(region), Some(iam_role), Some(period)) = (
        &options.aws_region,
        &options.aws_iam_kms_role,
        options.aws_sts_refresh_period_secs,
    ) {
        aws_session_tokens::get_session_token(region, period as i32, iam_role).await?
    } else {
        // Use dummy credentials for local development
        use crate::aws_session_tokens::AwsCredentials;
        AwsCredentials {
            access_key_id: "dummy-access-key".to_string(),
            secret_access_key: "dummy-secret-key".to_string(),
            session_token: Some("dummy-session-token".to_string()),
        }
    };

    // Create the application state
    let relayer_rpc_controller = RelayerRpcController::new(options.relayer_rpc_url.clone());
    let app_state = Arc::new(AppState {
        options,
        tee_task_pool,
        db_pool,
        aws_credentials: Arc::new(Mutex::new(aws_credentials)),
        relayer_rpc_controller,
    });

    // Perform initial TEE public key verification to ensure the server is correctly configured
    info!("Performing initial TEE public key verification...");
    let _verification_result =
        server_handlers::tee_public_key::tee_public_key(axum::extract::State(app_state.clone()))
            .await
            .map_err(|e| Error::ParseError(format!("TEE public key verification failed: {}", e)))?;

    info!("TEE public key verification successful");

    // Start the AWS credentials refresh task
    let credentials_refresh_state = app_state.clone();
    tokio::spawn(async move {
        aws_credentials_refresh_task(credentials_refresh_state).await;
    });

    // Start the scheduler processor
    let scheduler_processor = SchedulerProcessor::new(app_state.clone());
    tokio::spawn(async move {
        scheduler_processor.start().await;
    });

    let listener = TcpListener::bind((
        app_state.options.bind_address.clone(),
        app_state.options.public_port,
    ))
    .await?;

    // Set up the application routes
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
    serve(listener, router).await?;

    Ok(())
}

/// Background task that periodically refreshes AWS STS credentials
async fn aws_credentials_refresh_task(app_state: Arc<AppState>) {
    #[cfg(not(feature = "local-run"))]
    {
        let mut interval = tokio::time::interval(Duration::from_secs(
            app_state.options.aws_sts_refresh_period_secs,
        ));

        // Skip the first tick since we already have initial credentials
        interval.tick().await;

        loop {
            interval.tick().await;

            info!("Refreshing AWS credentials from EC2 metadata");
            match aws_session_tokens::get_session_token(
                &app_state.options.aws_region,
                app_state.options.aws_sts_refresh_period_secs as i32,
                &app_state.options.aws_iam_kms_role,
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
                    // Continue running - don't crash the server on credential refresh failure
                    // The old credentials might still be valid for a while
                }
            }
        }
    }

    #[cfg(feature = "local-run")]
    {
        // In local-run mode, only refresh credentials if AWS settings are provided
        if let (Some(region), Some(iam_role), Some(period)) = (
            &app_state.options.aws_region,
            &app_state.options.aws_iam_kms_role,
            app_state.options.aws_sts_refresh_period_secs,
        ) {
            let mut interval = tokio::time::interval(Duration::from_secs(period));

            // Skip the first tick since we already have initial credentials
            interval.tick().await;

            loop {
                interval.tick().await;

                info!("Refreshing AWS credentials from EC2 metadata (local-run mode)");
                match aws_session_tokens::get_session_token(region, period as i32, iam_role).await {
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
                        // Continue running - don't crash the server on credential refresh failure
                        // The old credentials might still be valid for a while
                    }
                }
            }
        } else {
            info!("AWS credentials refresh disabled in local-run mode (no AWS settings provided)");
            // Just keep the task alive but do nothing
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        }
    }
}
