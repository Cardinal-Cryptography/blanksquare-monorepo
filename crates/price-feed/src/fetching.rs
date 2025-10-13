use std::{future::Future, pin::Pin};

use alloy_primitives::{Address, U256};
use alloy_provider::ProviderBuilder;
use alloy_sol_types::sol;
use rust_decimal::{prelude::FromPrimitive, Decimal, MathematicalOps as _};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::PriceProvider;

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

#[derive(Clone, Debug, Deserialize)]
struct PythResult {
    parsed: Vec<PythItem>,
}

#[derive(Clone, Debug, Deserialize)]
struct PythItem {
    price: PythPrice,
}

#[derive(Clone, Debug, Deserialize)]
struct PythPrice {
    price: String,
    #[serde(rename = "expo")]
    exponent: i8,
    publish_time: u64,
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

impl TryFrom<PythResult> for PriceInfoFromProvider {
    type Error = PriceFetchError;

    fn try_from(value: PythResult) -> Result<Self, Self::Error> {
        if let [item] = &value.parsed[..] {
            let price = item
                .price
                .price
                .parse::<Decimal>()
                .map_err(|_| PriceFetchError::UnexpectedPythResponse)?;
            let exponent = item.price.exponent;
            let token_price = price * Decimal::new(10, 0).powd(exponent.into());
            let time = OffsetDateTime::from_unix_timestamp(item.price.publish_time as i64)
                .map_err(|_| PriceFetchError::UnexpectedPythResponse)?;

            return Ok(PriceInfoFromProvider { token_price, time });
        }

        Err(PriceFetchError::UnexpectedPythResponse)
    }
}

pub fn fetch_price(
    provider: &PriceProvider,
) -> Pin<Box<dyn Future<Output = Result<PriceInfoFromProvider, PriceFetchError>> + Send + '_>> {
    Box::pin(async move {
        match provider {
            PriceProvider::Dia(url) => fetch_dia(url).await,
            PriceProvider::Pyth(url) => fetch_pyth(url).await,
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
                fetch_erc4626(
                    underlying_price_provider,
                    *underlying_decimals,
                    node_rpc_url,
                    vault_address,
                    *vault_decimals,
                )
                .await
            }
        }
    })
}

async fn fetch_pyth(url: &str) -> Result<PriceInfoFromProvider, PriceFetchError> {
    reqwest::get(url)
        .await?
        .json::<PythResult>()
        .await?
        .try_into()
}

async fn fetch_dia(url: &str) -> Result<PriceInfoFromProvider, PriceFetchError> {
    Ok(reqwest::get(url)
        .await?
        .json::<PriceInfoFromProvider>()
        .await?)
}

async fn fetch_erc4626(
    underlying: &PriceProvider,
    underlying_decimals: u32,
    node_rpc_url: &str,
    vault_address: &Address,
    vault_decimals: u32,
) -> Result<PriceInfoFromProvider, PriceFetchError> {
    // get the udnerlying price in USD
    let underlying_price = fetch_price(underlying).await?;

    let provider = ProviderBuilder::new()
        .on_builtin(node_rpc_url)
        .await
        .map_err(|e| PriceFetchError::Provider(e.to_string()))?;

    // get the price per share (in underlying token) from the ERC4626 contract
    let one_share = 10u128.pow(vault_decimals);
    let token_per_share = ERC4626::new(*vault_address, provider.clone())
        .convertToAssets(U256::from(one_share))
        .call()
        .await?
        .assets;

    // Note: it will panic if the `token_per_share` is too big to fit into u128 which is very unlikely
    let one_underlying = 10u128.pow(underlying_decimals);
    let token_per_share_decimal =
        Decimal::from_u128(token_per_share.to::<u128>()).ok_or_else(|| {
            PriceFetchError::Provider("U256 to Decimal conversion failed".to_string())
        })? / Decimal::from_u128(one_underlying).unwrap();

    // finally calculate the vault token price in USD
    let token_price = token_per_share_decimal * underlying_price.token_price;

    Ok(PriceInfoFromProvider {
        token_price,
        time: underlying_price.time,
    })
}

sol! {
    #[sol(rpc)] // <-- Important! Generates the necessary `MyContract` struct and function methods.
    interface ERC4626 {
        #[derive(Debug)]
        function convertToAssets(uint256 shares) public view returns(uint256 assets);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy_primitives::Address;

    use super::fetch_price;
    use crate::PriceProvider;

    const ETH: &str =
        "https://api.diadata.org/v1/assetQuotation/Ethereum/0x0000000000000000000000000000000000000000";
    const USDT: &str =
        "https://api.diadata.org/v1/assetQuotation/Ethereum/0xdAC17F958D2ee523a2206206994597C13D831ec7";
    const USDC: &str =
        "https://api.diadata.org/v1/assetQuotation/Ethereum/0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    const HYPER: &str =
        "https://hermes.pyth.network/v2/updates/price/latest?ids%5B%5D=0x4279e31cc369bbcc2faf022b382b080e32a8e689ff20fbc530d2a603eb6cd98b";

    #[tokio::test]
    async fn can_fetch_price_from_dia() {
        for token in &[ETH, USDT] {
            fetch_price(&PriceProvider::Dia(token.to_string()))
                .await
                .expect("Should connect to the feed and get price");
        }
    }

    #[tokio::test]
    async fn can_fetch_price_from_pyth() {
        fetch_price(&PriceProvider::Pyth(HYPER.to_string()))
            .await
            .expect("Should be able to fetch price from pyth");
    }

    #[tokio::test]
    async fn can_fetch_price_from_vault() {
        fetch_price(&PriceProvider::ERC4626 {
            underlying_price_provider: Box::new(PriceProvider::Dia(USDC.to_string())),
            underlying_decimals: 6,
            node_rpc_url: "https://base.llamarpc.com".to_string(),
            vault_address: Address::from_str("0x0d877Dc7C8Fa3aD980DfDb18B48eC9F8768359C4").unwrap(),
            vault_decimals: 8,
        })
        .await
        .expect("Should be able to fetch price from vault");
    }
}
