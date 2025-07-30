use axum::extract::State;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::error::ShielderProverServerError;

pub fn register() -> Result<PrometheusHandle, ShielderProverServerError> {
    let builder = PrometheusBuilder::new();
    let handle = builder.install_recorder()?;
    Ok(handle)
}

#[axum::debug_handler]
pub async fn metrics(State(handle): State<PrometheusHandle>) -> String {
    handle.render()
}
