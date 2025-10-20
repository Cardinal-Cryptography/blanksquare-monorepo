use std::{future::Future, pin::Pin};

use alloy_primitives::{Address, U256};
use alloy_provider::ProviderBuilder;
use alloy_sol_types::sol;
use rust_decimal::{prelude::FromPrimitive, Decimal};

use super::{PriceFetchError, PriceInfoFromProvider};
use crate::price_provider::PriceProvider;

sol! {
    #[sol(rpc)]
    interface ERC4626 {
        function convertToAssets(uint256 shares) public view returns(uint256 assets);
    }
}

pub(crate) fn fetch_erc4626<'a>(
    underlying_price_provider: &'a PriceProvider,
    underlying_decimals: u32,
    node_rpc_url: &'a str,
    vault_address: &'a Address,
    vault_decimals: u32,
) -> Pin<Box<dyn Future<Output = Result<PriceInfoFromProvider, PriceFetchError>> + Send + 'a>> {
    Box::pin(async move {
        // get the underlying price in USD
        let underlying_price = underlying_price_provider.fetch_price().await?;

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

        let one_underlying = 10u128.pow(underlying_decimals);
        let token_per_share_decimal =
            Decimal::from_u128(token_per_share.to::<u128>()).ok_or_else(|| {
                PriceFetchError::Provider("U256 to Decimal conversion failed".to_string())
            })? / Decimal::from_u128(one_underlying).ok_or_else(|| {
                PriceFetchError::Provider("u128 to Decimal conversion failed".to_string())
            })?;

        // finally calculate the vault token price in USD
        let token_price = token_per_share_decimal * underlying_price.token_price;
        Ok(PriceInfoFromProvider {
            token_price,
            time: underlying_price.time,
        })
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy_primitives::Address;

    use crate::{price_provider::erc4626::fetch_erc4626, PriceProvider};

    const USDC: &str =
    "https://api.diadata.org/v1/assetQuotation/Ethereum/0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

    #[tokio::test]
    #[ignore] // external RPC + requires a live ERC4626 vault; run manually
    async fn can_fetch_price_from_vault() {
        fetch_erc4626(
            &PriceProvider::Dia(USDC.to_string()),
            6,
            "https://base.llamarpc.com",
            &Address::from_str("0x0d877Dc7C8Fa3aD980DfDb18B48eC9F8768359C4").unwrap(),
            8,
        )
        .await
        .expect("Should be able to fetch price from vault");
    }
}
