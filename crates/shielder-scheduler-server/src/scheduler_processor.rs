use std::{sync::Arc, time::Duration};

use alloy_primitives::{Address, U256};
use alloy_signer_local::PrivateKeySigner;
use chrono::Utc;
use shielder_contract::{merkle_path::get_current_merkle_path, ConnectionPolicy, ShielderUser};
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
    db::{
        get_pending_requests, update_request_status, update_retry_attempt, RequestStatus,
        ScheduledRequest,
    },
    error::SchedulerServerError,
    handlers::tee_request,
    AppState,
};

type Result<T> = std::result::Result<T, SchedulerServerError>;

/// Background request scheduler that processes scheduled withdrawal requests
#[derive(Debug)]
pub struct SchedulerProcessor {
    app_state: Arc<AppState>,
}

#[derive(Debug)]
struct ProcessingResult {
    request_id: i64,
}

impl SchedulerProcessor {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    /// Start the background processing loop
    pub async fn start(self) {
        info!("Starting background task processor");
        let mut interval = interval(Duration::from_secs(
            self.app_state.options.scheduler_interval_secs,
        ));

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
            self.app_state.options.shielder_address.parse().expect(
                "Failed to parse shielder_address as a valid Ethereum address. \
Please check the SHIELDER_ADDRESS environment variable or --shielder-address argument.",
            ),
            ConnectionPolicy::OnDemand {
                rpc_url: self.app_state.options.node_rpc_url.clone(),
                signer: PrivateKeySigner::random(),
            },
        )
    }

    async fn process_pending_requests(&self) -> Result<()> {
        let requests = get_pending_requests(
            &self.app_state.db_pool,
            self.app_state.options.scheduler_batch_size as i64,
        )
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
        info!("Processing request ID: {}", request.id);
        let request_id = request.id;
        let request_retry_count = request.retry_count;

        match self.process_request_logic(request).await {
            Ok(result) => {
                info!("Successfully processed request ID: {}", result.request_id);
                update_request_status(
                    &self.app_state.db_pool,
                    result.request_id,
                    RequestStatus::Completed,
                    None,
                )
                .await?;
            }
            Err(e) => {
                warn!(
                    "Request processing failed for ID: {}, error: {:?}",
                    request_id, e
                );

                if request_retry_count < self.app_state.options.scheduler_max_retry_count as i32 {
                    let new_relay_after = Utc::now()
                        + Duration::from_secs(self.app_state.options.scheduler_retry_delay_secs);

                    update_retry_attempt(
                        &self.app_state.db_pool,
                        request_id,
                        new_relay_after,
                        Some(&e.to_string()),
                    )
                    .await?;
                } else {
                    warn!(
                        "Request ID {} has reached maximum retry count, marking as Failed",
                        request_id
                    );
                    update_request_status(
                        &self.app_state.db_pool,
                        request_id,
                        RequestStatus::Failed,
                        Some(&e.to_string()),
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// The actual processing logic for a scheduled request
    async fn process_request_logic(&self, request: ScheduledRequest) -> Result<ProcessingResult> {
        // Parse the U256 values
        let last_note_index = request.last_note_index_as_u256().map_err(|e| {
            SchedulerServerError::ValueParseError(format!("Failed to parse last_note_index: {}", e))
        })?;
        let max_relayer_fee = request.max_relayer_fee_as_u256().map_err(|e| {
            SchedulerServerError::ValueParseError(format!("Failed to parse max_relayer_fee: {}", e))
        })?;
        let pocket_money = request.pocket_money_as_u256().map_err(|e| {
            SchedulerServerError::ValueParseError(format!("Failed to parse pocket_money: {}", e))
        })?;
        let token_address = request.token_address_as_address().map_err(|e| {
            SchedulerServerError::ValueParseError(format!("Failed to parse token_address: {}", e))
        })?;

        let (merkle_root, merkle_path) = self.current_merkle_path(last_note_index).await?;

        // Send PrepareRelayCalldata request to TEE server
        let tee_task_pool = self.app_state.tee_task_pool.clone();
        let app_state = self.app_state.clone();
        let payload = request.payload.clone();
        
        let response = tee_task_pool
            .spawn(async move {
                // Create the PrepareRelayCalldata request
                let request = Request::PrepareRelayCalldata {
                    payload,
                    relayer_fee: max_relayer_fee,
                    relayer_address: Address::ZERO, // TODO: Get actual relayer address
                    merkle_path: Box::new(merkle_path),
                    merkle_root,
                    pocket_money,
                    protocol_fee: U256::ZERO, // TODO: Calculate protocol fee based on token_address
                };

                // Send request to TEE using the existing tee_request function
                let json_response = tee_request(app_state, request).await?;
                Ok::<Response, VsockError>(json_response.0)
            })
            .await
            .map_err(SchedulerServerError::TaskPool)?
            .await
            .map_err(SchedulerServerError::JoinHandleError)??
            .map_err(SchedulerServerError::ProvingServerError)?;

        // Handle the response
        match response {
            Response::PrepareRelayCalldata { calldata } => {
                info!(
                    "Successfully prepared relay calldata for request ID: {} - amount: {}, merkle_root: {}",
                    request.id, calldata.amount, calldata.merkle_root
                );
                // TODO: Submit to relayer or relay directly to the blockchain
            }
            _ => {
                return Err(SchedulerServerError::ProvingServerError(
                    VsockError::Protocol("Unexpected response from TEE".to_string()),
                ));
            }
        }

        info!(
            "Processing withdrawal request - last_note_index: {}, max_relayer_fee: {}, pocket_money: {}, token_address: {}, relay_after: {}",
            last_note_index, max_relayer_fee, pocket_money, token_address, request.relay_after
        );

        Ok(ProcessingResult {
            request_id: request.id,
        })
    }
}
