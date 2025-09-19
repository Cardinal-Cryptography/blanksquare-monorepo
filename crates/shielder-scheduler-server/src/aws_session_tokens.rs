use serde::Deserialize;
use tracing::{info, instrument};

use crate::error::SchedulerServerError;

/// AWS credentials retrieved from EC2 instance metadata
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Response structure from EC2 instance metadata service
#[derive(Debug, Deserialize)]
struct Ec2MetadataResponse {
    #[serde(rename = "Code")]
    code: String,
    #[serde(rename = "AccessKeyId")]
    access_key_id: String,
    #[serde(rename = "SecretAccessKey")]
    secret_access_key: String,
    #[serde(rename = "Token")]
    token: String,
}

/// Retrieves AWS credentials from EC2 instance metadata service
#[instrument(level = "info")]
pub async fn get_session_token(
    refresh_period_seconds: i32,
    iam_role_name: &str,
) -> Result<AwsCredentials, SchedulerServerError> {
    use std::time::Duration;

    use aws_config::imds::client::Client;

    info!("Retrieving AWS credentials from EC2 instance metadata");

    // Calculate token TTL as twice the refresh period for safety margin
    let token_ttl = Duration::from_secs((refresh_period_seconds * 2).max(60) as u64);

    // Create IMDS client with appropriate configuration
    let client = Client::builder()
        .max_attempts(3)
        .token_ttl(token_ttl)
        .build();

    // Get the credentials using the AWS IMDS client
    let credentials_path = format!(
        "/latest/meta-data/iam/security-credentials/{}",
        iam_role_name
    );

    let credentials_response = client.get(credentials_path).await.map_err(|e| {
        SchedulerServerError::AwsError(format!("Failed to get credentials from IMDS: {}", e))
    })?;

    // Convert SensitiveString to String for parsing
    let credentials_text: String = credentials_response.into();

    // Parse the JSON response
    let metadata_response: Ec2MetadataResponse =
        serde_json::from_str(&credentials_text).map_err(|e| {
            SchedulerServerError::AwsError(format!("Failed to parse credentials JSON: {}", e))
        })?;

    // Validate the response
    if metadata_response.code != "Success" {
        return Err(SchedulerServerError::AwsError(format!(
            "Metadata service returned error code: {}",
            metadata_response.code
        )));
    }

    // Validate credentials are not empty
    if metadata_response.access_key_id.is_empty() {
        return Err(SchedulerServerError::AwsError(
            "Access key ID is empty".to_string(),
        ));
    }

    if metadata_response.secret_access_key.is_empty() {
        return Err(SchedulerServerError::AwsError(
            "Secret access key is empty".to_string(),
        ));
    }

    if metadata_response.token.is_empty() {
        return Err(SchedulerServerError::AwsError(
            "Session token is empty".to_string(),
        ));
    }

    info!("Successfully retrieved AWS credentials from EC2 metadata service");

    Ok(AwsCredentials {
        access_key_id: metadata_response.access_key_id,
        secret_access_key: metadata_response.secret_access_key,
        session_token: Some(metadata_response.token),
    })
}
