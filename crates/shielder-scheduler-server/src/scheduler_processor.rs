use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::time::interval;
use tracing::{error, info, instrument, warn};

use crate::{
    db::{
        get_pending_requests, update_request_status, update_retry_attempt, RequestStatus,
        ScheduledRequest,
    },
    AppState,
};

/// Background request scheduler that processes scheduled withdrawal requests
#[derive(Debug)]
pub struct SchedulerProcessor {
    app_state: Arc<AppState>,
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

    async fn process_pending_requests(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get pending requests that are ready to be processed
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
    async fn process_single_request(
        &self,
        request: ScheduledRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Processing request ID: {}", request.id);
        let request_id = request.id;
        let request_retry_count = request.retry_count;

        match process_request_logic(request).await {
            Ok(result) => {
                info!("Successfully processed request ID: {}", result.request_id);
                // Mark request as completed
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
                    // Increment retry count
                    let new_relay_after = Utc::now()
                        + Duration::from_secs(self.app_state.options.scheduler_retry_delay_secs);

                    // Update retry attempt
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
                    // Mark request as failed
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
}

#[derive(Debug)]
struct ProcessingResult {
    request_id: i64,
}

/// The actual processing logic for a scheduled request
async fn process_request_logic(
    request: ScheduledRequest,
) -> Result<ProcessingResult, Box<dyn std::error::Error + Send + Sync>> {
    // Parse the U256 values
    let last_note_index = request
        .last_note_index_as_u256()
        .map_err(|e| format!("Failed to parse last_note_index: {}", e))?;
    let max_relayer_fee = request
        .max_relayer_fee_as_u256()
        .map_err(|e| format!("Failed to parse max_relayer_fee: {}", e))?;

    // TODO:
    // 1. Get the current merkle path for the note
    // 2. Get fee quote from the relayer
    // 3. Prepare the relay calldata using the TEE
    // 4. Submit to a relayer or relay directly to the blockchain

    // For now, we'll simulate the processing
    info!(
        "Processing withdrawal request - last_note_index: {}, max_relayer_fee: {}, relay_after: {}",
        last_note_index, max_relayer_fee, request.relay_after
    );

    // For now, just return success
    Ok(ProcessingResult {
        request_id: request.id,
    })
}
