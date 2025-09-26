use std::sync::Arc;

use axum::Json;
use shielder_scheduler_common::{
    protocol::{Request, Response, TEEClient},
    vsock::VsockError,
};
use tracing::{info_span, Instrument as _};

use crate::{storage::StorageInterface, AppState};

pub mod get_status;
pub mod health;
pub mod schedule_withdraw;
pub mod tee_public_key;

/// Sends a request to the TEE server and returns the response.
pub async fn tee_request<Storage: StorageInterface>(
    state: Arc<AppState<Storage>>,
    request: Request,
) -> Result<Json<Response>, VsockError> {
    let mut tee_client = TEEClient::new(state.options.tee_cid, state.options.tee_port)
        .instrument(info_span!("Building_VSOCK_connection"))
        .await?;

    let response = tee_client
        .request(&request)
        .instrument(info_span!("Sending_TEE_request"))
        .await?;

    Ok(Json(response))
}
