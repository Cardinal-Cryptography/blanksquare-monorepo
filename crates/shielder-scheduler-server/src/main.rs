mod command_line_args;
mod db;
mod error;
mod handlers;
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
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::{
    command_line_args::CommandLineArgs,
    handlers::{self as server_handlers},
    scheduler_processor::SchedulerProcessor,
};

#[derive(Debug)]
struct AppState {
    options: CommandLineArgs,
    db_pool: db::PgPool,
    tee_task_pool: Arc<tokio_task_pool::Pool>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Parse command line arguments
    let options = CommandLineArgs::parse();

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

    // Create the application state
    let app_state = Arc::new(AppState {
        options,
        tee_task_pool,
        db_pool,
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
