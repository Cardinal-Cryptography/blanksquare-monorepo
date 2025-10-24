use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use anyhow::Result;
use shielder_account::{ShielderAccount, Token};
use shielder_circuits::poseidon::off_circuit::hash;
use shielder_contract::{
    call_type::{Call, DryRun},
    providers::create_simple_provider,
    recovery::get_shielder_action,
    ShielderUser,
};
use shielder_setup::protocol_fee::compute_protocol_fee_from_gross;
use tracing::info;
use type_conversions::{field_to_u256, u256_to_field};

use crate::shielder::new_account::create_new_account;

pub async fn ensure_account_created(
    shielder_user: &ShielderUser,
    shielder_seed: U256,
    rpc_url: String,
    contract_address: Address,
    token: Token,
    amount: U256,
) -> Result<ShielderAccount> {
    let amount = U256::from(amount);

    let mut shielder_account = ShielderAccount::new(shielder_seed, token);

    recover_state(&mut shielder_account, shielder_user, &rpc_url).await?;

    let protocol_fee_bps = shielder_user.protocol_deposit_fee_bps::<DryRun>().await?;
    let protocol_fee = compute_protocol_fee_from_gross(U256::from(amount), protocol_fee_bps);

    let provider = create_simple_provider(&rpc_url).await?;

    if let Token::ERC20(token_address) = token {
        let allowance = shielder_user
            .erc20_allowance::<DryRun>(token_address, shielder_user.address(), contract_address)
            .await?;
        let minimum_allowance = match shielder_account.nonce {
            0 => (amount + protocol_fee) * U256::from(2),
            _ => amount + protocol_fee,
        };
        if allowance < minimum_allowance {
            let (tx_hash, _) = shielder_user
                .approve_erc20::<Call>(token_address, contract_address, minimum_allowance)
                .await?;
            provider
                .get_transaction_receipt(tx_hash)
                .await?
                .expect("Transaction receipt not found");
        }
    }

    if shielder_account.nonce == 0 {
        info!("Account is not created yet. Creating a new account...");
        let tx_hash = create_new_account(
            &shielder_account,
            shielder_user,
            amount,
            token,
            protocol_fee,
        )
        .await?;

        provider
            .get_transaction_receipt(tx_hash)
            .await?
            .expect("Transaction receipt not found");

        recover_state(&mut shielder_account, shielder_user, &rpc_url).await?;
    }
    Ok(shielder_account)
}

async fn recover_state(
    account: &mut ShielderAccount,
    shielder_user: &ShielderUser,
    rpc_url: &str,
) -> Result<()> {
    let provider = create_simple_provider(rpc_url).await?;

    loop {
        let expected_nullifier = account.previous_nullifier();
        let expected_nullifier_hash = field_to_u256(hash(&[u256_to_field(expected_nullifier)]));

        match get_shielder_action(&provider, shielder_user, expected_nullifier_hash).await? {
            Some(action) => account.register_action(action),
            None => break,
        }
    }
    Ok(())
}
