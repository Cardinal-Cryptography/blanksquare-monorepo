use aws_sdk_sts::{config::BehaviorVersion, Client};
use tracing::{info, instrument};

use crate::error::SchedulerServerError;

/// AWS credentials retrieved from STS
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Retrieves AWS session token using the STS GetSessionToken API
#[instrument(level = "info")]
pub async fn get_session_token(aws_region: &str, duration_seconds: i32) -> Result<AwsCredentials, SchedulerServerError> {
    info!("Retrieving AWS session token using STS with duration {} seconds", duration_seconds);

    // Create STS client with the provided region
    let config = aws_sdk_sts::Config::builder()
        .region(aws_sdk_sts::config::Region::new(aws_region.to_string()))
        .behavior_version(BehaviorVersion::latest())
        .build();
    let client = Client::from_conf(config);

    // Call GetSessionToken with specified duration
    let result = client
        .get_session_token()
        .set_duration_seconds(Some(duration_seconds))
        .send()
        .await
        .map_err(|e| {
            SchedulerServerError::AwsError(format!("Failed to get session token: {}", e))
        })?;

    let credentials = result.credentials().ok_or_else(|| {
        SchedulerServerError::AwsError("No credentials returned from STS".to_string())
    })?;

    let access_key_id = credentials.access_key_id();
    let secret_access_key = credentials.secret_access_key();
    let session_token = credentials.session_token();

    info!("Successfully retrieved AWS session token");

    Ok(AwsCredentials {
        access_key_id: access_key_id.to_string(),
        secret_access_key: secret_access_key.to_string(),
        session_token: Some(session_token.to_string()),
    })
}
