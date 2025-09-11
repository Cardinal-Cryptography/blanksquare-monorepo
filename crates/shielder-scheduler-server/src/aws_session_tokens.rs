#[cfg(not(feature = "local-run"))]
use serde::Deserialize;
use tracing::{info, instrument};

#[cfg(not(feature = "local-run"))]
use reqwest;
#[cfg(not(feature = "local-run"))]
use tracing::debug;

use crate::error::SchedulerServerError;

/// AWS credentials retrieved from EC2 instance metadata
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Response structure from EC2 instance metadata service
#[cfg(not(feature = "local-run"))]
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
#[cfg(not(feature = "local-run"))]
#[instrument(level = "info")]
pub async fn get_session_token(_aws_region: &str, refresh_period_seconds: i32, iam_role_name: &str) -> Result<AwsCredentials, SchedulerServerError> {
    info!("Retrieving AWS credentials from EC2 instance metadata");

    // Step 1: Get the metadata token
    let token_url = "http://169.254.169.254/latest/api/token";
    let client = reqwest::Client::new();
    
    // Calculate TTL as twice the refresh period
    let token_ttl = refresh_period_seconds * 2;
    
    let token_response = client
        .put(token_url)
        .header("X-aws-ec2-metadata-token-ttl-seconds", token_ttl.to_string())
        .send()
        .await
        .map_err(|e| {
            SchedulerServerError::AwsError(format!("Failed to get metadata token: {}", e))
        })?;

    if !token_response.status().is_success() {
        return Err(SchedulerServerError::AwsError(format!(
            "Failed to get metadata token, status: {}",
            token_response.status()
        )));
    }

    let token = token_response.text().await.map_err(|e| {
        SchedulerServerError::AwsError(format!("Failed to read metadata token: {}", e))
    })?;

    // Step 2: Get the credentials using the token
    let credentials_url = format!("http://169.254.169.254/latest/meta-data/iam/security-credentials/{}", iam_role_name);
    
    let credentials_response = client
        .get(credentials_url)
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .await
        .map_err(|e| {
            SchedulerServerError::AwsError(format!("Failed to get credentials: {}", e))
        })?;

    if !credentials_response.status().is_success() {
        return Err(SchedulerServerError::AwsError(format!(
            "Failed to get credentials, status: {}",
            credentials_response.status()
        )));
    }

    let credentials_text = credentials_response.text().await.map_err(|e| {
        SchedulerServerError::AwsError(format!("Failed to read credentials response: {}", e))
    })?;

    // Step 3: Parse the JSON response
    let metadata_response: Ec2MetadataResponse = serde_json::from_str(&credentials_text)
        .map_err(|e| {
            SchedulerServerError::AwsError(format!("Failed to parse credentials JSON: {}", e))
        })?;

    // Step 4: Validate the response
    if metadata_response.code != "Success" {
        return Err(SchedulerServerError::AwsError(format!(
            "Metadata service returned error code: {}",
            metadata_response.code
        )));
    }

    // Step 5: Validate credentials are not empty
    if metadata_response.access_key_id.is_empty() {
        return Err(SchedulerServerError::AwsError(
            "Access key ID is empty".to_string()
        ));
    }
    
    if metadata_response.secret_access_key.is_empty() {
        return Err(SchedulerServerError::AwsError(
            "Secret access key is empty".to_string()
        ));
    }
    
    if metadata_response.token.is_empty() {
        return Err(SchedulerServerError::AwsError(
            "Session token is empty".to_string()
        ));
    }

    // Step 6: Debug log credentials (first 4 characters only for security)
    debug!(
        "Retrieved AWS credentials - Access Key: {}..., Secret Key: {}..., Token: {}...",
        &metadata_response.access_key_id.chars().take(4).collect::<String>(),
        &metadata_response.secret_access_key.chars().take(4).collect::<String>(),
        &metadata_response.token.chars().take(4).collect::<String>()
    );

    info!("Successfully retrieved AWS credentials from EC2 metadata service");

    Ok(AwsCredentials {
        access_key_id: metadata_response.access_key_id,
        secret_access_key: metadata_response.secret_access_key,
        session_token: Some(metadata_response.token),
    })
}

/// Returns dummy AWS credentials for local development
#[cfg(feature = "local-run")]
#[instrument(level = "info")]
pub async fn get_session_token(_aws_region: &str, _refresh_period_seconds: i32, _iam_role_name: &str) -> Result<AwsCredentials, SchedulerServerError> {
    info!("Using dummy AWS credentials for local development");

    Ok(AwsCredentials {
        access_key_id: "dummy_access_key_id".to_string(),
        secret_access_key: "dummy_secret_access_key".to_string(),
        session_token: Some("dummy_session_token".to_string()),
    })
}
