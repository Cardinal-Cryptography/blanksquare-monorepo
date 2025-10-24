use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use anyhow::Result;
use shielder_account::{ShielderAccount, Token};
use shielder_contract::{providers::create_simple_provider, ShielderUser};

use crate::shielder::{deposit::estimate_deposit_gas, new_account::estimate_new_account_gas};

#[derive(Clone, serde::Serialize)]
pub struct FeeResponse {
    pub native_new_account_gas: String,
    pub native_deposit_gas: String,
    pub erc20_new_account_gas: String,
    pub erc20_deposit_gas: String,
    pub gas_price_native: String,
    pub update_timestamp: i64,
}

/// Returns a FeeResponse with gas estimations computed concurrently.
pub async fn get_fee_values(
    created_shielder_account_native: &ShielderAccount,
    created_shielder_account_erc20: &ShielderAccount,
    empty_shielder_seed: &U256,
    erc20_token_address: Address,
    shielder_user: &ShielderUser,
    rpc_url: &str,
) -> Result<FeeResponse> {
    let provider = create_simple_provider(rpc_url).await?;
    let empty_shielder_account_native = ShielderAccount::new(*empty_shielder_seed, Token::Native);
    let empty_shielder_account_erc20 =
        ShielderAccount::new(*empty_shielder_seed, Token::ERC20(erc20_token_address));
    // Run all gas estimations and gas price fetch concurrently
    let (
        native_new_account_result,
        erc20_new_account_result,
        native_deposit_result,
        erc20_deposit_result,
        gas_price_result,
    ) = tokio::join!(
        estimate_new_account_gas(
            &empty_shielder_account_native,
            shielder_user,
            Token::Native,
            U256::from(1),
        ),
        estimate_new_account_gas(
            &empty_shielder_account_erc20,
            shielder_user,
            Token::ERC20(erc20_token_address),
            U256::from(1),
        ),
        estimate_deposit_gas(
            created_shielder_account_native,
            shielder_user,
            Token::Native,
            U256::from(1),
        ),
        estimate_deposit_gas(
            created_shielder_account_erc20,
            shielder_user,
            Token::ERC20(erc20_token_address),
            U256::from(1),
        ),
        provider.get_gas_price()
    );

    // Handle results and propagate any errors
    let native_new_account_gas = native_new_account_result?.to_string();
    let erc20_new_account_gas = erc20_new_account_result?.to_string();
    let native_deposit_gas = native_deposit_result?.to_string();
    let erc20_deposit_gas = erc20_deposit_result?.to_string();
    let gas_price_native = gas_price_result?.to_string();

    Ok(FeeResponse {
        native_new_account_gas,
        native_deposit_gas,
        erc20_new_account_gas,
        erc20_deposit_gas,
        gas_price_native,
        update_timestamp: time::OffsetDateTime::now_utc().unix_timestamp(),
    })
}
