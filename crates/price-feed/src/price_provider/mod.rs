use alloy_primitives::Address;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub(crate) mod dia;
pub(crate) mod erc4626;
pub(crate) mod pyth;

/// This is the struct that we expect to receive at `https://api.diadata.org/v1/assetQuotation/`.
#[derive(Clone, Debug, Deserialize)]
pub struct PriceInfoFromProvider {
    #[serde(rename = "Price")]
    pub token_price: Decimal,
    #[serde(
        rename = "Time",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    pub time: OffsetDateTime,
}

#[derive(thiserror::Error, Debug)]
pub enum PriceFetchError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Unexpected pyth response")]
    UnexpectedPythResponse,

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Contract call error: {0}")]
    ContractCall(#[from] alloy_contract::Error),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum PriceProvider {
    Dia(String),
    Pyth(String),
    Static(Decimal),
    ERC4626 {
        underlying_price_provider: Box<PriceProvider>,
        underlying_decimals: u32,
        node_rpc_url: String,
        vault_address: Address,
        vault_decimals: u32,
    },
}

impl PriceProvider {
    pub async fn fetch_price(&self) -> Result<PriceInfoFromProvider, PriceFetchError> {
        match self {
            PriceProvider::Dia(url) => dia::fetch_dia(url).await,
            PriceProvider::Pyth(url) => pyth::fetch_pyth(url).await,
            PriceProvider::Static(price) => Ok(PriceInfoFromProvider {
                token_price: *price,
                time: OffsetDateTime::now_utc(),
            }),
            PriceProvider::ERC4626 {
                underlying_price_provider,
                underlying_decimals,
                node_rpc_url,
                vault_address,
                vault_decimals,
            } => {
                erc4626::fetch_erc4626(
                    underlying_price_provider,
                    *underlying_decimals,
                    node_rpc_url,
                    vault_address,
                    *vault_decimals,
                )
                .await
            }
        }
    }
}
