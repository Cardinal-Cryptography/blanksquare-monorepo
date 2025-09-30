use std::sync::Arc;

use alloy_primitives::{Address, U256};
use alloy_signer_local::PrivateKeySigner;
use chrono::Utc;
use shielder_contract::{merkle_path::get_current_merkle_path, ConnectionPolicy, ShielderUser};
use shielder_relayer::{QuoteFeeResponse, RelayQuery};
use shielder_scheduler_common::{
    protocol::{Request, Response},
    vsock::VsockError,
};
use shielder_setup::{
    consts::ARITY, shielder_circuits::consts::merkle_constants::NOTE_TREE_HEIGHT,
};
use tokio::time::interval;
use tracing::{error, info, instrument, warn};

use crate::{
    app_state::AppState,
    credentials_provider::CredentialsProvider,
    error::SchedulerServerError,
    storage::{RequestStatus, ScheduledRequest, StorageProvider},
    ENCRYPTION_ALGORITHM,
};

type Result<T> = std::result::Result<T, SchedulerServerError>;

/// Background request scheduler that processes scheduled withdrawal requests
#[derive(Debug)]
pub struct SchedulerProcessor<
    Storage: StorageProvider + 'static,
    Credentials: CredentialsProvider + 'static,
> {
    app_state: Arc<AppState<Storage, Credentials>>,
    scheduler_interval_secs: u64,
    scheduler_batch_size: usize,
    scheduler_max_retry_count: usize,
    scheduler_retry_delay_secs: u32,
    shielder_address: Address,
    node_rpc_url: String,
}

impl<Storage: StorageProvider, Credentials: CredentialsProvider>
    SchedulerProcessor<Storage, Credentials>
{
    pub fn new(
        app_state: Arc<AppState<Storage, Credentials>>,
        scheduler_interval_secs: u64,
        scheduler_batch_size: usize,
        scheduler_max_retry_count: usize,
        scheduler_retry_delay_secs: u32,
        shielder_address: Address,
        node_rpc_url: String,
    ) -> Self {
        Self {
            app_state,
            scheduler_interval_secs,
            scheduler_batch_size,
            scheduler_max_retry_count,
            scheduler_retry_delay_secs,
            shielder_address,
            node_rpc_url,
        }
    }

    pub async fn start(self) {
        info!("Starting background task processor");
        let mut interval = interval(std::time::Duration::from_secs(self.scheduler_interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.process_pending_requests().await {
                error!("Error processing pending requests: {:?}", e);
            }
        }
    }

    async fn current_merkle_path(
        &self,
        leaf_index: U256,
    ) -> Result<(U256, [[U256; ARITY]; NOTE_TREE_HEIGHT])> {
        let shielder_user = self.shielder_user_read_only();
        Ok(get_current_merkle_path(leaf_index, &shielder_user).await?)
    }

    fn shielder_user_read_only(&self) -> ShielderUser {
        ShielderUser::new(
            self.shielder_address,
            ConnectionPolicy::OnDemand {
                rpc_url: self.node_rpc_url.clone(),
                signer: PrivateKeySigner::random(),
            },
        )
    }

    async fn process_pending_requests(&self) -> Result<()> {
        let requests = self
            .app_state
            .storage
            .get_pending_requests(self.scheduler_batch_size)
            .await?;

        if requests.is_empty() {
            return Ok(());
        }

        info!("Processing {} pending requests", requests.len());

        for request in requests {
            if let Err(e) = self.process_single_request(request).await {
                error!("Failed to process request: {:?}", e);
            }
        }

        Ok(())
    }

    #[instrument(level = "info", skip_all)]
    async fn process_single_request(&self, request: ScheduledRequest) -> Result<()> {
        info!(
            "Processing request last_note_index: {}",
            request.last_note_index
        );
        let request_retry_count = request.retry_count;

        match self.process_request_logic(&request).await {
            Ok(_) => {
                info!(
                    "Successfully processed request last_note_index: {}",
                    request.last_note_index
                );
                self.app_state
                    .storage
                    .update_request_status(
                        &request.last_note_index.to_string(),
                        RequestStatus::Completed,
                        Some(Utc::now()),
                        None,
                    )
                    .await?;
            }
            Err(e) => {
                warn!(
                    "Request processing failed for last_note_index: {}, error: {:?}",
                    request.last_note_index, e
                );

                if request_retry_count < self.scheduler_max_retry_count as u8 {
                    let new_relay_after = Utc::now()
                        + chrono::Duration::seconds(self.scheduler_retry_delay_secs as i64);
                    let new_retry_count = request_retry_count + 1;
                    self.app_state
                        .storage
                        .update_retry_attempt(
                            &request.last_note_index.to_string(),
                            new_relay_after,
                            new_retry_count,
                            Some(Utc::now()),
                            Some(&e.to_string()),
                        )
                        .await?;
                } else {
                    warn!(
                        "Request last_note_index {} has reached maximum retry count, marking as Failed",
                        request.last_note_index
                    );
                    self.app_state
                        .storage
                        .update_request_status(
                            &request.last_note_index.to_string(),
                            RequestStatus::Failed,
                            Some(Utc::now()),
                            Some(&e.to_string()),
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn process_request_logic(&self, request: &ScheduledRequest) -> Result<()> {
        let (merkle_root, merkle_path) = self.current_merkle_path(request.last_note_index).await?;
        let quoted_fee = self
            .get_quoted_fee(request.token_address, request.pocket_money)
            .await?;

        let tee_response = self
            .call_tee_prepare_relay_calldata(request, &quoted_fee, merkle_root, merkle_path)
            .await?;

        match tee_response {
            Response::PrepareRelayCalldata { calldata } => {
                info!(
                    "Successfully prepared relay calldata for request last_note_index: {}",
                    request.last_note_index,
                );
                let relay_query = RelayQuery {
                    calldata,
                    quote: quoted_fee.into(),
                };
                self.app_state
                    .relayer_controller
                    .send_relay_query(relay_query)
                    .await?;
                Ok(())
            }
            _ => Err(SchedulerServerError::ProvingServerError(
                VsockError::Protocol("Unexpected response from TEE".to_string()),
            )),
        }
    }

    async fn get_quoted_fee(
        &self,
        token_address: Address,
        pocket_money: U256,
    ) -> Result<QuoteFeeResponse> {
        let token = if token_address == Address::ZERO {
            shielder_account::Token::Native
        } else {
            shielder_account::Token::ERC20(token_address)
        };

        self.app_state
            .relayer_controller
            .get_relayer_total_fee(token, pocket_money)
            .await
    }

    async fn call_tee_prepare_relay_calldata(
        &self,
        request: &ScheduledRequest,
        quoted_fee: &QuoteFeeResponse,
        merkle_root: U256,
        merkle_path: [[U256; ARITY]; NOTE_TREE_HEIGHT],
    ) -> Result<Response> {
        let encryption_envelope = request.encryption_envelope.clone();
        let relayer_address = self
            .app_state
            .relayer_controller
            .get_relayer_fee_address()
            .await?;

        let relayer_fee = quoted_fee.fee_details.total_cost_fee_token;

        // Get current AWS credentials
        let aws_credentials = self.app_state.credentials.get_credentials().await?;

        let request = Request::PrepareRelayCalldata {
            aws_config: Box::new(shielder_scheduler_common::protocol::AwsConfig {
                public_key: self.app_state.kms_public_key.clone(),
                kms_key_id: self.app_state.kms_key_id.clone(),
                aws_region: aws_credentials.region,
                aws_access_key_id: aws_credentials.access_key_id,
                aws_secret_access_key: aws_credentials.secret_access_key,
                aws_session_token: aws_credentials.session_token.unwrap_or_default(),
                kms_encryption_algorithm: ENCRYPTION_ALGORITHM.to_string(),
            }),
            encryption_envelope: Box::new(encryption_envelope),
            relayer_address,
            merkle_path: Box::new(merkle_path),
            merkle_root,
            relayer_fee,
        };

        self.app_state.tee_controller.tee_request(request).await
    }
}
