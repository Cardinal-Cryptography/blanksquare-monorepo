use alloy_primitives::{Address, Bytes, TxHash, U256};
use anyhow::Result;
use shielder_account::{
    call_data::{NewAccountCall, NewAccountCallExtra, NewAccountCallType},
    ShielderAccount, Token,
};
use shielder_contract::{
    call_type::{Call, DryRun, EstimateGas},
    ShielderUser,
};
use shielder_setup::{
    protocol_fee::compute_protocol_fee_from_gross, shielder_circuits::GrumpkinPointAffine,
};

use crate::shielder::{get_mac_salt, pk::NEW_ACCOUNT_PROVING_EQUIPMENT};

pub async fn estimate_new_account_gas(
    shielder_account: &ShielderAccount,
    shielder_user: &ShielderUser,
    token: Token,
    amount: U256,
) -> Result<u64> {
    let amount = U256::from(amount);

    let anonymity_revoker_public_key = shielder_user.anonymity_revoker_pubkey::<DryRun>().await?;
    let protocol_fee_bps = shielder_user.protocol_deposit_fee_bps::<DryRun>().await?;

    let protocol_fee = compute_protocol_fee_from_gross(amount, protocol_fee_bps);

    let call = prepare_call(
        shielder_account,
        amount,
        token,
        anonymity_revoker_public_key,
        shielder_user.address(),
        protocol_fee,
        Bytes::from(vec![]),
    )?;
    let estimated_gas = match token {
        Token::Native => {
            shielder_user
                .new_account_native::<EstimateGas>(call.try_into().unwrap(), amount)
                .await?
        }
        Token::ERC20(_) => {
            shielder_user
                .new_account_erc20::<EstimateGas>(call.try_into().unwrap())
                .await?
        }
    };

    Ok(estimated_gas)
}

fn prepare_call(
    shielder_account: &ShielderAccount,
    amount: U256,
    token: Token,
    anonymity_revoker_public_key: GrumpkinPointAffine<U256>,
    caller_address: Address,
    protocol_fee: U256,
    memo: Bytes,
) -> Result<NewAccountCall> {
    let (params, pk) = NEW_ACCOUNT_PROVING_EQUIPMENT.clone();
    // let (params, pk) = get_proving_equipment(CircuitType::NewAccount)?;
    let extra = NewAccountCallExtra {
        anonymity_revoker_public_key,
        encryption_salt: get_mac_salt(),
        mac_salt: get_mac_salt(),
        caller_address,
        protocol_fee,
        memo,
    };

    Ok(shielder_account.prepare_call::<NewAccountCallType>(&params, &pk, token, amount, &extra))
}

pub async fn create_new_account(
    shielder_account: &ShielderAccount,
    user: &ShielderUser,
    amount: U256,
    token: Token,
    protocol_fee: U256,
) -> Result<TxHash> {
    let anonymity_revoker_public_key = user.anonymity_revoker_pubkey::<DryRun>().await?;

    let call = prepare_call(
        shielder_account,
        amount,
        token,
        anonymity_revoker_public_key,
        user.address(),
        protocol_fee,
        Bytes::from(vec![]),
    )?;

    let (tx_hash, _) = match token {
        Token::Native => {
            user.new_account_native::<Call>(call.try_into().unwrap(), amount)
                .await?
        }
        Token::ERC20(_) => {
            user.new_account_erc20::<Call>(call.try_into().unwrap())
                .await?
        }
    };

    Ok(tx_hash)
}
