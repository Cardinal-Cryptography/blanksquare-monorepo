use crate::{
    credentials_provider::{AwsCredentials, CredentialsProvider},
    error::SchedulerServerError,
};

pub struct DummyCredentialsProvider;

impl CredentialsProvider for DummyCredentialsProvider {
    async fn get_credentials(&self) -> Result<AwsCredentials, SchedulerServerError> {
        Ok(AwsCredentials {
            access_key_id: "dummy-access-key".to_string(),
            secret_access_key: "dummy-secret-key".to_string(),
            session_token: Some("dummy-session-token".to_string()),
            region: "dummy_region".to_string(),
        })
    }
}
