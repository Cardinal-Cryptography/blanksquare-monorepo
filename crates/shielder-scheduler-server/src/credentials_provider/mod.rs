pub mod aws_credentials_provider;
pub mod dummy_credentials_provider;

#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub region: String,
}

pub trait CredentialsProvider: Send + Sync {
    fn get_credentials(&self) -> impl std::future::Future<Output = AwsCredentials> + Send;
}
