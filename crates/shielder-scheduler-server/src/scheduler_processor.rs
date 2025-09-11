use std::{sync::Arc, time::Duration};

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
    db::{
        get_pending_requests, update_request_status, update_retry_attempt, RequestStatus,
        ScheduledRequest,
    },
    error::SchedulerServerError,
    handlers::tee_request,
    AppState,
};

type Result<T> = std::result::Result<T, SchedulerServerError>;

#[derive(Debug)]
struct ParsedRequestParams {
    last_note_index: U256,
    max_relayer_fee: U256,
    pocket_money: U256,
    token_address: Address,
}

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

    async fn process_request_logic(&self, request: ScheduledRequest) -> Result<ProcessingResult> {
        let parsed_params = self.parse_request_parameters(&request)?;
        let (merkle_root, merkle_path) = self
            .current_merkle_path(parsed_params.last_note_index)
            .await?;
        let quoted_fee = self
            .get_quoted_fee(parsed_params.token_address, parsed_params.pocket_money)
            .await?;

        self.validate_fee_within_limit(&quoted_fee, parsed_params.max_relayer_fee)?;

        let tee_response = self
            .call_tee_prepare_relay_calldata(
                &request,
                &quoted_fee,
                merkle_root,
                merkle_path,
                parsed_params.pocket_money,
            )
            .await?;

        self.process_tee_response(tee_response, quoted_fee, request.id)
            .await
    }

    fn parse_request_parameters(&self, request: &ScheduledRequest) -> Result<ParsedRequestParams> {
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

        Ok(ParsedRequestParams {
            last_note_index,
            max_relayer_fee,
            pocket_money,
            token_address,
        })
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
            .relayer_rpc_controller
            .get_relayer_total_fee(token, pocket_money)
            .await
    }

    fn validate_fee_within_limit(
        &self,
        quoted_fee: &QuoteFeeResponse,
        max_relayer_fee: U256,
    ) -> Result<()> {
        if quoted_fee.fee_details.total_cost_fee_token > max_relayer_fee {
            return Err(SchedulerServerError::ProvingServerError(
                VsockError::Protocol(format!(
                    "Relayer fee {} exceeds maximum allowed {}",
                    quoted_fee.fee_details.total_cost_fee_token, max_relayer_fee
                )),
            ));
        }
        Ok(())
    }

    async fn call_tee_prepare_relay_calldata(
        &self,
        request: &ScheduledRequest,
        quoted_fee: &QuoteFeeResponse,
        merkle_root: U256,
        merkle_path: [[U256; ARITY]; NOTE_TREE_HEIGHT],
        pocket_money: U256,
    ) -> Result<Response> {
        let tee_task_pool = self.app_state.tee_task_pool.clone();
        let app_state = self.app_state.clone();
        let payload = request.payload.clone();
        let relayer_address = self
            .app_state
            .relayer_rpc_controller
            .get_relayer_fee_address()
            .await?;

        let max_relayer_fee = quoted_fee.fee_details.total_cost_fee_token;

        let response = tee_task_pool
            .spawn(async move {
                let request = Request::PrepareRelayCalldata {
                    payload,
                    max_relayer_fee,
                    relayer_address,
                    merkle_path: Box::new(merkle_path),
                    merkle_root,
                    pocket_money,
                };

                let json_response = tee_request(app_state, request).await?;
                Ok::<Response, VsockError>(json_response.0)
            })
            .await
            .map_err(SchedulerServerError::TaskPool)?
            .await
            .map_err(SchedulerServerError::JoinHandleError)??
            .map_err(SchedulerServerError::ProvingServerError)?;

        Ok(response)
    }

    async fn process_tee_response(
        &self,
        response: Response,
        quoted_fee: QuoteFeeResponse,
        request_id: i64,
    ) -> Result<ProcessingResult> {
        match response {
            Response::PrepareRelayCalldata { calldata } => {
                info!(
                    "Successfully prepared relay calldata for request ID: {}",
                    request_id,
                );
                let relay_query = RelayQuery {
                    calldata,
                    quote: quoted_fee.into(),
                };
                self.app_state
                    .relayer_rpc_controller
                    .send_relay_query(relay_query)
                    .await?;
                Ok(ProcessingResult { request_id })
            }
            _ => Err(SchedulerServerError::ProvingServerError(
                VsockError::Protocol("Unexpected response from TEE".to_string()),
            )),
        }
    }
}
