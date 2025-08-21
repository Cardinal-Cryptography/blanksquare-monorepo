use std::sync::Arc;

use axum::Json;
use shielder_scheduler_common::{
    metrics::FutureTimingMetric::*,
    protocol::{Request, Response, TEEClient},
    vsock::VsockError,
};
use tracing::{info_span, Instrument as _};

use crate::AppState;

pub mod health;
pub mod schedule_withdraw;
pub mod tee_public_key;

/// Sends a request to the TEE server and returns the response.
async fn tee_request(state: Arc<AppState>, request: Request) -> Result<Json<Response>, VsockError> {
    let mut tee_client = TEEClient::new(state.options.tee_cid, state.options.tee_port as u32)
        .instrument(info_span!(BuildingVsocksConnection.name()))
        .await?;

    let response = tee_client
        .request(&request)
        .instrument(info_span!(SendingTeeRequest.name()))
        .await?;

    Ok(Json(response))
}
