use alloy_primitives::{Address, Bytes, U256};
use anyhow::Result;
use shielder_account::{
    call_data::{DepositCall, DepositCallType, DepositExtra},
    ShielderAccount, Token,
};
use shielder_contract::{
    call_type::{DryRun, EstimateGas},
    merkle_path::get_current_merkle_path,
    ShielderUser,
};
use shielder_setup::{
    consts::{ARITY, TREE_HEIGHT},
    protocol_fee::compute_protocol_fee_from_gross,
};

use crate::shielder::{get_mac_salt, pk::DEPOSIT_PROVING_EQUIPMENT};

pub async fn estimate_deposit_gas(
    shielder_account: &ShielderAccount,
    shielder_user: &ShielderUser,
    token: Token,
    amount: U256,
) -> Result<u64> {
    let amount = U256::from(amount);

    let protocol_fee_bps = shielder_user.protocol_deposit_fee_bps::<DryRun>().await?;
    let protocol_fee = compute_protocol_fee_from_gross(U256::from(amount), protocol_fee_bps);

    let leaf_index = shielder_account
        .current_leaf_index()
        .expect("Deposit mustn't be the first action");
    let (_merkle_root, merkle_path) = get_current_merkle_path(leaf_index, shielder_user).await?;

    let call = prepare_call(
        shielder_account,
        amount,
        token,
        merkle_path,
        shielder_user.address(),
        protocol_fee,
        Bytes::from(vec![]),
    )?;
    let estimated_gas = match token {
        Token::Native => {
            shielder_user
                .deposit_native::<EstimateGas>(call.try_into().unwrap(), amount)
                .await?
        }
        Token::ERC20(_) => {
            shielder_user
                .deposit_erc20::<EstimateGas>(call.try_into().unwrap())
                .await?
        }
    };
    Ok(estimated_gas)
}

fn prepare_call(
    shielder_account: &ShielderAccount,
    amount: U256,
    token: Token,
    merkle_path: [[U256; ARITY]; TREE_HEIGHT],
    caller_address: Address,
    protocol_fee: U256,
    memo: Bytes,
) -> Result<DepositCall> {
    let (params, pk) = DEPOSIT_PROVING_EQUIPMENT.clone();
    let extra = DepositExtra {
        merkle_path,
        mac_salt: get_mac_salt(),
        caller_address,
        protocol_fee,
        memo,
    };

    Ok(shielder_account.prepare_call::<DepositCallType>(&params, &pk, token, amount, &extra))
}
