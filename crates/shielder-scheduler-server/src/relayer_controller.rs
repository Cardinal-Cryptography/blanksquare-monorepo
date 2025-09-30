use std::str::FromStr;

use alloy_primitives::{Address, TxHash, U256};
use serde::{Deserialize, Serialize};
use shielder_relayer::{
    QuoteFeeQuery, QuoteFeeResponse, RelayQuery, RelayResponse, SimpleServiceResponse,
};
use tracing::debug;

use crate::error::SchedulerServerError;

/// Controller for interacting with the relayer service via its API.
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct RelayerController {
    base_url: String,
}

impl RelayerController {
    /// Create a new RelayerController with the given base URL.
    /// Example: "https://base-testnet-shielder-relayer-v3.test.blanksquare.dev"
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub fn healthcheck_url(&self) -> String {
        format!("{}/health", self.base_url)
    }

    pub fn relay_url(&self) -> String {
        format!("{}/relay", self.base_url)
    }

    fn fees_url(&self) -> String {
        format!("{}/quote_fees", self.base_url)
    }

    pub fn fee_address_url(&self) -> String {
        format!("{}/fee_address", self.base_url)
    }

    pub async fn health_check(&self) -> Result<(), SchedulerServerError> {
        let response = reqwest::get(self.healthcheck_url()).await.map_err(|e| {
            SchedulerServerError::RelayerError(format!(
                "Failed to perform relayer health check: {}",
                e
            ))
        })?;
        if response.status().is_success() {
            debug!("Relayer health check succeeded.");
            Ok(())
        } else {
            Err(SchedulerServerError::RelayerError(format!(
                "Relayer health check failed with status: {}",
                response.status()
            )))
        }
    }

    pub async fn get_relayer_fee_address(&self) -> Result<Address, SchedulerServerError> {
        let relayer_response = reqwest::Client::new()
            .get(self.fee_address_url())
            .send()
            .await
            .map_err(|e| {
                SchedulerServerError::RelayerError(format!(
                    "Failed to get relayer fee address: {}",
                    e
                ))
            })?;

        if !relayer_response.status().is_success() {
            return Err(SchedulerServerError::RelayerError(format!(
                "Failed to get relayer fee address: {:?}",
                relayer_response.status()
            )));
        }
        let address = relayer_response
            .json::<SimpleServiceResponse>()
            .await
            .map_err(|e| {
                SchedulerServerError::RelayerError(format!(
                    "Failed to parse relayer fee address response: {}",
                    e
                ))
            })?
            .message;
        Address::from_str(&address).map_err(|e| {
            SchedulerServerError::RelayerError(format!(
                "Failed to parse relayer fee address: {}",
                e
            ))
        })
    }

    pub async fn get_relayer_total_fee(
        &self,
        token: shielder_account::Token,
        pocket_money: U256,
    ) -> Result<QuoteFeeResponse, SchedulerServerError> {
        let relayer_response = reqwest::Client::new()
            .post(self.fees_url())
            .json(&QuoteFeeQuery {
                fee_token: token,
                pocket_money,
            })
            .send()
            .await
            .map_err(|e| {
                SchedulerServerError::RelayerError(format!("Failed to query relayer fees: {}", e))
            })?;

        if !relayer_response.status().is_success() {
            return Err(SchedulerServerError::RelayerError(format!(
                "Relayer failed to quote fees: {:?}",
                relayer_response.status()
            )));
        }
        let quoted_fees = relayer_response
            .json::<QuoteFeeResponse>()
            .await
            .map_err(|e| {
                SchedulerServerError::RelayerError(format!(
                    "Failed to parse relayer fees response: {}",
                    e
                ))
            })?;
        Ok(quoted_fees)
    }

    pub async fn send_relay_query(
        &self,
        relay_query: RelayQuery,
    ) -> Result<TxHash, SchedulerServerError> {
        let relayer_response = reqwest::Client::new()
            .post(self.relay_url())
            .json(&relay_query)
            .send()
            .await
            .map_err(|e| {
                SchedulerServerError::RelayerError(format!("Failed to send relay query: {}", e))
            })?;

        if !relayer_response.status().is_success() {
            return Err(SchedulerServerError::RelayerError(format!(
                "Relayer failed to process relay query: {:?}",
                relayer_response.status()
            )));
        }
        let relay_response: RelayResponse = relayer_response.json().await.map_err(|e| {
            SchedulerServerError::RelayerError(format!("Failed to parse relayer response: {}", e))
        })?;
        let tx_hash = relay_response.tx_hash;

        if tx_hash == TxHash::ZERO {
            return Err(SchedulerServerError::RelayerError(
                "Relayer returned invalid transaction hash: zero hash".to_string(),
            ));
        }

        Ok(tx_hash)
    }
}
