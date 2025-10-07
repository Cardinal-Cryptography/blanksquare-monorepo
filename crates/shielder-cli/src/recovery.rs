use alloy_primitives::U256;
use anyhow::Result;
use shielder_account::Token;
use shielder_circuits::poseidon::off_circuit::hash;
use shielder_contract::{providers::create_simple_provider, recovery::get_shielder_action};
use type_conversions::{field_to_u256, u256_to_field};

use crate::app_state::AppState;

pub async fn recover_state(
    app_state: &mut AppState,
    token: Token,
    zkid_seed: Option<U256>,
) -> Result<()> {
    let shielder_user = app_state.create_shielder_user();
    app_state.ensure_account_exist(token, zkid_seed);
    let AppState {
        accounts,
        node_rpc_url,
        ..
    } = app_state;
    let provider = create_simple_provider(node_rpc_url).await?;

    let account = accounts
        .get_mut(&token.address())
        .expect("We have just ensured the account exists");

    loop {
        let expected_nullifier = account.previous_nullifier();
        let expected_nullifier_hash = field_to_u256(hash(&[u256_to_field(expected_nullifier)]));

        match get_shielder_action(&provider, &shielder_user, expected_nullifier_hash).await? {
            Some(action) => account.register_action(action),
            None => break,
        }
    }

    // also recover scheduler accounts for this token
    recover_scheduler_accounts(app_state, token, zkid_seed).await?;
    Ok(())
}

pub async fn recover_scheduler_accounts(
    app_state: &mut AppState,
    token: Token,
    zkid_seed: Option<U256>,
) -> Result<()> {
    let shielder_user = app_state.create_shielder_user();
    let node_rpc_url = &app_state.node_rpc_url;
    let provider = create_simple_provider(node_rpc_url).await?;

    let zkid_seed = zkid_seed.unwrap_or_else(|| app_state.default_zkid_seed(token));
    let scheduler_accounts_entry = app_state.scheduler_accounts.entry(zkid_seed).or_default();
    for scheduler_account in scheduler_accounts_entry.iter_mut() {
        loop {
            let expected_nullifier = scheduler_account.previous_nullifier();
            let expected_nullifier_hash = field_to_u256(hash(&[u256_to_field(expected_nullifier)]));

            match get_shielder_action(&provider, &shielder_user, expected_nullifier_hash).await? {
                Some(action) => scheduler_account.register_action(action),
                None => break,
            }
        }
    }

    // Recover new scheduler accounts until no more actions are found
    loop {
        let mut scheduler_account = app_state.get_next_scheduler_account(token, zkid_seed);
        let mut found_action = false;
        loop {
            let expected_nullifier = scheduler_account.previous_nullifier();
            let expected_nullifier_hash = field_to_u256(hash(&[u256_to_field(expected_nullifier)]));

            match get_shielder_action(&provider, &shielder_user, expected_nullifier_hash).await? {
                Some(action) => {
                    scheduler_account.register_action(action);
                    found_action = true;
                }
                None => break,
            }
        }
        if found_action {
            app_state.scheduler_accounts.entry(zkid_seed).or_default().push(scheduler_account);
        } else {
            break;
        }
    }
    Ok(())
}
