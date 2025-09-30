use aws_config::SdkConfig;
use aws_sdk_dynamodb::config::{ProvideCredentials, SharedCredentialsProvider};

use crate::{
    credentials_provider::{AwsCredentials, CredentialsProvider},
    error::SchedulerServerError,
};

pub struct AwsCredentialsProvider {
    credentials_provider: SharedCredentialsProvider,
    region: String,
}

impl AwsCredentialsProvider {
    pub async fn new(aws_sdk_config: &SdkConfig) -> Result<Self, SchedulerServerError> {
        let credentials_provider =
            aws_sdk_config
                .credentials_provider()
                .ok_or(SchedulerServerError::CredentialsError(
                    "No credentials provider found in AWS SDK config".to_string(),
                ))?;
        let region = aws_sdk_config
            .region()
            .ok_or(SchedulerServerError::CredentialsError(
                "No region found in AWS SDK config".to_string(),
            ))?
            .to_string();
        Ok(Self {
            credentials_provider,
            region,
        })
    }
}

impl CredentialsProvider for AwsCredentialsProvider {
    async fn get_credentials(&self) -> Result<AwsCredentials, SchedulerServerError> {
        let credentials = self
            .credentials_provider
            .provide_credentials()
            .await
            .map_err(|e| {
                SchedulerServerError::CredentialsError(format!(
                    "Failed to retrieve AWS credentials: {}",
                    e
                ))
            })?;

        Ok(AwsCredentials {
            access_key_id: credentials.access_key_id().to_string(),
            secret_access_key: credentials.secret_access_key().to_string(),
            session_token: credentials.session_token().map(|s| s.to_string()),
            region: self.region.clone(),
        })
    }
}
