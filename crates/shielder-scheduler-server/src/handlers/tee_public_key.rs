use std::sync::Arc;

use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use shielder_scheduler_common::protocol::{Request, Response};
use tracing::instrument;

use crate::{error::SchedulerServerError, handlers::tee_request, AppState};

#[instrument(level = "info", skip_all)]
pub async fn tee_public_key(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Response>, SchedulerServerError> {
    let tee_task_pool = state.tee_task_pool.clone();
    let state_cloned = state.clone();
    let public_key = BASE64
        .decode(state_cloned.options.kms_public_key.clone())
        .map_err(|e| {
            SchedulerServerError::ParseError(format!(
                "Failed to decode KMS_PUBLIC_KEY from base64: {e:?}"
            ))
        })?;
    tee_task_pool
        .spawn(async move {
            tee_request(
                state_cloned.clone(),
                Request::TeePublicKey {
                    aws_config: shielder_scheduler_common::protocol::AwsConfig {
                        public_key,
                        kms_key_id: state_cloned.options.kms_key_id.clone(),
                        aws_region: state_cloned.options.aws_region.clone(),
                        aws_access_key_id: state_cloned.options.aws_access_key_id.clone(),
                        aws_secret_access_key: state_cloned.options.aws_secret_access_key.clone(),
                        aws_session_token: state_cloned.options.aws_session_token.clone(),
                        kms_encryption_algorithm: state_cloned
                            .options
                            .kms_encryption_algorithm
                            .clone(),
                    },
                },
            )
            .await
        })
        .await
        .map_err(SchedulerServerError::TaskPool)?
        .await
        .map_err(SchedulerServerError::JoinHandleError)??
        .map_err(SchedulerServerError::ProvingServerError)
}
