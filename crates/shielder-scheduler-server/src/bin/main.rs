use std::{net::SocketAddrV4, sync::Arc, time::Duration};

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use clap::Parser;
use metrics_exporter_prometheus::PrometheusBuilder;
use shielder_scheduler_common::metrics::FutureHistogramLayer;
use shielder_scheduler_server::{
    app_state::AppState,
    config::Config,
    credentials_provider,
    error::SchedulerServerError as Error,
    handlers::{self as server_handlers},
    scheduler_processor::SchedulerProcessor,
    storage,
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let options = Config::parse();
    let kms_key_id = options.kms_key_id.clone().ok_or(Error::ParseError(
        "KMS_KEY_ID environment variable is required".to_string(),
    ))?;

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

    let aws_sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

    let storage = storage::dynamo_db::DynamoDb::new(&aws_sdk_config, &options.node_rpc_url).await?;

    let aws_credentials_provider =
        credentials_provider::aws_credentials_provider::AwsCredentialsProvider::new(
            &aws_sdk_config,
        )
        .await?;

    let app_state = Arc::new(AppState::new(
        kms_key_id,
        options.clone(),
        aws_credentials_provider,
        storage,
    ));

    // Perform initial TEE public key verification to ensure the server is correctly configured
    info!("Performing initial TEE public key verification...");
    let _verification_result =
        server_handlers::tee_public_key::tee_public_key(axum::extract::State(app_state.clone()))
            .await
            .map_err(|e| Error::ParseError(format!("TEE public key verification failed: {}", e)))?;

    info!("TEE public key verification successful");

    let scheduler_processor = SchedulerProcessor::new(
        app_state.clone(),
        options.scheduler_interval_secs,
        options.scheduler_batch_size,
        options.scheduler_max_retry_count,
        options.scheduler_retry_delay_secs,
        options.shielder_address,
        options.node_rpc_url.clone(),
    );
    tokio::spawn(async move {
        scheduler_processor.start().await;
    });

    let listener = TcpListener::bind((options.bind_address, options.public_port)).await?;

    let app = Router::new()
        .route("/health", get(server_handlers::health::health))
        .route(
            "/public_key",
            get(server_handlers::tee_public_key::tee_public_key),
        )
        .route(
            "/schedule_withdraw",
            post(server_handlers::schedule_withdraw::schedule_withdraw),
        )
        .route(
            "/status/{last_note_index}",
            get(server_handlers::get_status::get_status),
        )
        .layer(DefaultBodyLimit::max(options.maximum_request_size))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    info!("Starting server on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
