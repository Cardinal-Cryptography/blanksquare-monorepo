use crate::{
    credentials_provider::CredentialsProvider, relayer_controller::RelayerController,
    storage::StorageProvider, tee_controller::TeeController,
};

#[derive(Debug)]
pub struct AppState<Storage: StorageProvider, Credentials: CredentialsProvider> {
    pub kms_key_id: String,
    pub kms_public_key: String,
    pub credentials: Credentials,
    pub storage: Storage,
    pub relayer_controller: RelayerController,
    pub tee_controller: TeeController,
}

impl<Storage: StorageProvider, Credentials: CredentialsProvider> AppState<Storage, Credentials> {
    pub fn new(
        kms_key_id: String,
        configuration: crate::config::Config,
        credentials_provider: Credentials,
        storage_provider: Storage,
    ) -> AppState<Storage, Credentials> {
        let relayer_controller = RelayerController::new(configuration.relayer_url.clone());
        let tee_controller = TeeController::new(
            configuration.tee_port,
            configuration.tee_cid,
            configuration.tee_task_pool_capacity,
            configuration.tee_task_pool_timeout_secs,
            configuration.tee_compute_timeout_secs,
        );

        AppState {
            kms_key_id,
            kms_public_key: configuration.kms_public_key.clone(),
            credentials: credentials_provider,
            storage: storage_provider,
            relayer_controller,
            tee_controller,
        }
    }
}
