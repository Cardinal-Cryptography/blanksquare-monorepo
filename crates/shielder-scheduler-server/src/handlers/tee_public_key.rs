use std::sync::Arc;

use axum::{extract::State, Json};
use shielder_scheduler_common::protocol::{Request, Response};
use tracing::instrument;

use crate::{
    app_state::AppState, credentials_provider::CredentialsProvider, error::SchedulerServerError,
    storage::StorageProvider, ENCRYPTION_ALGORITHM,
};

#[instrument(level = "info", skip_all)]
pub async fn tee_public_key<
    Storage: StorageProvider + 'static,
    Credentials: CredentialsProvider + 'static,
>(
    State(state): State<Arc<AppState<Storage, Credentials>>>,
) -> Result<Json<Response>, SchedulerServerError> {
    // Get current AWS credentials
    let aws_credentials = state.credentials.get_credentials().await?;
    let tee_response = state
        .tee_controller
        .tee_request(Request::TeePublicKey {
            aws_config: Box::new(shielder_scheduler_common::protocol::AwsConfig {
                public_key: state.kms_public_key.clone(),
                kms_key_id: state.kms_key_id.clone(),
                aws_region: aws_credentials.region,
                aws_access_key_id: aws_credentials.access_key_id,
                aws_secret_access_key: aws_credentials.secret_access_key,
                aws_session_token: aws_credentials.session_token.unwrap_or_default(),
                kms_encryption_algorithm: ENCRYPTION_ALGORITHM.to_string(),
            }),
        })
        .await?;
    Ok(Json(tee_response))
}
