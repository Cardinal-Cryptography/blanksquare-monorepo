use std::{sync::Arc, time::Duration};

use shielder_scheduler_common::protocol::{Request, Response, TEEClient};
use tracing::{info_span, Instrument as _};

use crate::error::SchedulerServerError;

/// Controller for interacting with the TEE via vsock.
#[derive(Debug)]
pub struct TeeController {
    tee_task_pool: Arc<tokio_task_pool::Pool>,
    tee_port: u32,
    tee_cid: u32,
}

impl TeeController {
    /// Create a new TeeController with the given TEE port and CID.
    pub fn new(
        tee_port: u32,
        tee_cid: u32,
        tee_task_pool_capacity: u16,
        tee_task_pool_timeout_secs: u64,
        tee_compute_timeout_secs: u64,
    ) -> Self {
        let tee_task_pool = tokio_task_pool::Pool::bounded(tee_task_pool_capacity as usize)
            .with_spawn_timeout(Duration::from_secs(tee_task_pool_timeout_secs))
            .with_run_timeout(Duration::from_secs(tee_compute_timeout_secs))
            .into();
        Self {
            tee_task_pool,
            tee_port,
            tee_cid,
        }
    }

    /// Sends a request to the TEE server and returns the response.
    pub async fn tee_request(&self, request: Request) -> Result<Response, SchedulerServerError> {
        let tee_cid = self.tee_cid;
        let tee_port = self.tee_port;
        let response = self
            .tee_task_pool
            .spawn(async move {
                let mut tee_client = TEEClient::new(tee_cid, tee_port)
                    .instrument(info_span!("Building_VSOCK_connection"))
                    .await?;
                let response = tee_client
                    .request(&request)
                    .instrument(info_span!("Sending_TEE_request"))
                    .await?;
                Ok(response)
            })
            .await
            .map_err(SchedulerServerError::TaskPool)?
            .await
            .map_err(SchedulerServerError::JoinHandleError)??
            .map_err(SchedulerServerError::VsockError)?;
        match response {
            Response::Error(e) => Err(SchedulerServerError::TeeError(e)),
            _ => Ok(response),
        }
    }
}
