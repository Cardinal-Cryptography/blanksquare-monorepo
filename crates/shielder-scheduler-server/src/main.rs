mod aws_session_tokens;
mod command_line_args;
mod db;
mod error;
mod handlers;
mod scheduler_processor;

pub use handlers::tee_prepare_relay_calldata::prepare_relay_calldata;

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
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::{
    aws_session_tokens::AwsCredentials,
    command_line_args::CommandLineArgs,
    handlers::{self as server_handlers},
    scheduler_processor::SchedulerProcessor,
};

#[derive(Debug)]
struct AppState {
    options: CommandLineArgs,
    db_pool: db::PgPool,
    tee_task_pool: Arc<tokio_task_pool::Pool>,
    aws_credentials: Arc<Mutex<AwsCredentials>>,
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

    // Get AWS session token using STS
    let aws_credentials = aws_session_tokens::get_session_token(
        &options.aws_region,
        (options.aws_sts_refresh_period_secs * 2) as i32
    ).await?;

    // Create the application state
    let app_state = Arc::new(AppState {
        options,
        tee_task_pool,
        db_pool,
        aws_credentials: Arc::new(Mutex::new(aws_credentials)),
    });

    // Perform initial TEE public key verification to ensure the server is correctly configured
    info!("Performing initial TEE public key verification...");
    let _verification_result = server_handlers::tee_public_key::tee_public_key(
        axum::extract::State(app_state.clone())
    ).await.map_err(|e| Error::ParseError(format!("TEE public key verification failed: {}", e)))?;

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
    let mut interval = tokio::time::interval(Duration::from_secs(
        app_state.options.aws_sts_refresh_period_secs,
    ));

    // Skip the first tick since we already have initial credentials
    interval.tick().await;

    loop {
        interval.tick().await;

        info!("Refreshing AWS STS credentials");
        match aws_session_tokens::get_session_token(
            &app_state.options.aws_region,
            (app_state.options.aws_sts_refresh_period_secs * 2) as i32
        ).await {
            Ok(new_credentials) => {
                let mut credentials = app_state.aws_credentials.lock().await;
                *credentials = new_credentials;
                info!("AWS STS credentials refreshed successfully");
            }
            Err(e) => {
                tracing::error!("Failed to refresh AWS STS credentials: {:?}", e);
                // Continue running - don't crash the server on credential refresh failure
                // The old credentials might still be valid for a while
            }
        }
    }
}
