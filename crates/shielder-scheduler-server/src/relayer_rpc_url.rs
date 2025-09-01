use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::SchedulerServerError;

/// The URL of the relayer RPC.
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]

pub struct RelayerRpcUrl {
	base_url: String,
}

impl RelayerRpcUrl {
	pub fn new(base_url: String) -> Self {
		Self { base_url }
	}

	pub fn healthcheck_url(&self) -> String {
		format!("{}/health", self.base_url)
	}

	pub fn relay_url(&self) -> String {
		format!("{}/relay", self.base_url)
	}

	pub fn fees_url(&self) -> String {
		format!("{}/quote_fees", self.base_url)
	}

	pub fn fee_address_url(&self) -> String {
		format!("{}/fee_address", self.base_url)
	}

	pub async fn check_connection(&self) -> Result<(), SchedulerServerError> {
		let response = reqwest::get(self.healthcheck_url()).await.map_err(|e| {
			SchedulerServerError::RelayerError(format!("Failed to check relayer health: {}", e))
		})?;
		if response.status().is_success() {
			debug!("Relayer healthcheck succeeded.");
			Ok(())
		} else {
			Err(SchedulerServerError::RelayerError(
				format!("Relayer healthcheck failed with status: {}", response.status()).into()
			))
		}
	}
}
