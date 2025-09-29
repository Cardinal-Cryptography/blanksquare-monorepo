use aws_config::SdkConfig;
use aws_sdk_dynamodb::config::ProvideCredentials;

use crate::credentials_provider::CredentialsProvider;

pub struct AwsCredentialsProvider {
    aws_sdk_config: SdkConfig,
}

impl AwsCredentialsProvider {
    pub async fn new() -> Self {
        let aws_sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Self { aws_sdk_config }
    }
}

impl CredentialsProvider for AwsCredentialsProvider {
    async fn get_credentials(&self) -> crate::credentials_provider::AwsCredentials {
        let credentials = self
            .aws_sdk_config
            .credentials_provider()
            .expect("No credentials provider configured")
            .provide_credentials()
            .await
            .expect("Failed to retrieve AWS credentials");

        crate::credentials_provider::AwsCredentials {
            access_key_id: credentials.access_key_id().to_string(),
            secret_access_key: credentials.secret_access_key().to_string(),
            session_token: credentials.session_token().map(|s| s.to_string()),
            region: self
                .aws_sdk_config
                .region()
                .expect("No region configured")
                .to_string(),
        }
    }
}
