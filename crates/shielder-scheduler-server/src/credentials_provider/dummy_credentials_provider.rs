use crate::credentials_provider::{AwsCredentials, CredentialsProvider};

pub struct DummyCredentialsProvider;

impl CredentialsProvider for DummyCredentialsProvider {
    async fn get_credentials(&self) -> AwsCredentials {
        AwsCredentials {
            access_key_id: "dummy-access-key".to_string(),
            secret_access_key: "dummy-secret-key".to_string(),
            session_token: Some("dummy-session-token".to_string()),
            region: "dummy_region".to_string(),
        }
    }
}
