use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::Provider;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use rsa::{pkcs8::DecodePublicKey, sha2::Sha256, Oaep, RsaPublicKey};
use serde::{Deserialize, Serialize};
use shielder_account::{
    call_data::{NewAccountCall, NewAccountCallExtra, NewAccountCallType},
    ShielderAccount, ShielderAction, Token,
};
use shielder_circuits::GrumpkinPointAffine;
use shielder_contract::{
    call_type::{Call, DryRun},
    events::get_event,
    ShielderContract::NewAccount,
};
use shielder_scheduler_common::protocol::EncryptionEnvelope;
use shielder_setup::{protocol_fee::compute_protocol_fee_from_net, version::contract_version};
use tracing::{debug, info, warn};

use crate::{
    app_state::AppState,
    shielder_ops::{
        get_mac_salt,
        pk::{get_proving_equipment, CircuitType},
    },
};

#[derive(Debug, Deserialize)]
struct TeePublicKeyResponse {
    #[serde(rename = "TeePublicKey")]
    tee_public_key: TeePublicKeyData,
}

#[derive(Debug, Deserialize)]
struct TeePublicKeyData {
    public_key: String,
    attestation_document: String,
}

#[derive(Debug, Serialize)]
struct ScheduleWithdrawRequest {
    encryption_envelope: EncryptionEnvelope,
    last_note_index: U256,
    relay_after: i64,
    pocket_money: U256,
    token_address: Address,
}

#[derive(Debug, Deserialize)]
struct ScheduleWithdrawResponse {
    request_id: String,
    message: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn schedule_withdraw(
    app_state: &mut AppState,
    amount: u128,
    to: Address,
    token: Token,
    pocket_money: u128,
    memo_deposit: Vec<u8>,
    memo_withdraw: Vec<u8>,
    relay_after_timestamp: i64,
    max_relayer_fee: u128,
    zkid_seed: Option<U256>,
) -> Result<()> {
    let amount_u256 = U256::from(amount);
    let zkid_seed = zkid_seed.unwrap_or_else(|| app_state.default_zkid_seed(token));
    // create shielder account from seed and index
    let mut scheduler_account = app_state.get_next_scheduler_account(token, zkid_seed);

    // compute total amount needed:
    // 1. A = amount
    // 2. RF = max_relayer_fee
    // 3. WP = withdraw protocol fee = compute_protocol_fee_from_net(A + RF)
    // 4. DP = deposit protocol fee = compute_protocol_fee_from_net(A + RF + WP)
    // TOTAL = A + RF + DP + WP
    // The `to` account will receive at least A and at most A + RF, paying TOTAL amount.
    let (total_amount, withdraw_protocol_fee, deposit_protocol_fee) = {
        let max_relayer_fee_u256 = U256::from(max_relayer_fee);
        let withdraw_protocol_fee_bps = if let Some(bps) = app_state.protocol_fees.withdraw_fee {
            bps
        } else {
            let user = app_state.create_shielder_user();
            let bps = user.protocol_withdraw_fee_bps::<DryRun>().await?;
            app_state.protocol_fees.withdraw_fee = Some(bps);
            bps
        };
        let withdraw_protocol_fee = compute_protocol_fee_from_net(
            amount_u256 + max_relayer_fee_u256,
            withdraw_protocol_fee_bps,
        );

        let deposit_protocol_fee_bps = if let Some(bps) = app_state.protocol_fees.deposit_fee {
            bps
        } else {
            let user = app_state.create_shielder_user();
            let bps = user.protocol_deposit_fee_bps::<DryRun>().await?;
            app_state.protocol_fees.deposit_fee = Some(bps);
            bps
        };
        let deposit_protocol_fee = compute_protocol_fee_from_net(
            amount_u256 + max_relayer_fee_u256 + withdraw_protocol_fee,
            deposit_protocol_fee_bps,
        );

        (
            amount_u256 + max_relayer_fee_u256 + withdraw_protocol_fee + deposit_protocol_fee,
            withdraw_protocol_fee,
            deposit_protocol_fee,
        )
    };

    // deposit into the scheduler account
    let last_note_index = deposit_call(
        app_state,
        &mut scheduler_account,
        total_amount,
        deposit_protocol_fee,
        token,
        memo_deposit,
    )
    .await?;

    // prepare payload for withdrawal
    let payload = prepare_withdrawal_payload(
        app_state,
        &scheduler_account,
        total_amount - deposit_protocol_fee,
        withdraw_protocol_fee,
        to,
        token,
        U256::from(pocket_money),
        memo_withdraw,
        last_note_index,
        U256::from(max_relayer_fee),
    )
    .await?;

    // get TEE public key
    let tee_public_key = get_tee_public_key(app_state).await?;

    // encrypt payload
    let encryption_envelope = encrypt_payload(&payload, &tee_public_key)?;

    // send schedule request
    send_schedule_request(
        app_state,
        encryption_envelope,
        last_note_index,
        pocket_money,
        token.address(),
        relay_after_timestamp,
    )
    .await?;

    // store the scheduler account back
    app_state
        .scheduler_accounts
        .entry(zkid_seed)
        .or_default()
        .push(scheduler_account);

    info!(
        "Withdrawal scheduled successfully for {} tokens to be relayed after timestamp {}",
        amount, relay_after_timestamp
    );
    Ok(())
}
async fn deposit_call(
    app_state: &mut AppState,
    scheduler_account: &mut ShielderAccount,
    amount: U256,
    protocol_fee: U256,
    token: Token,
    memo: Vec<u8>,
) -> Result<U256> {
    // call new_account using the scheduler account
    let memo = Bytes::from(memo);
    let user = app_state.create_shielder_user();
    let anonymity_revoker_public_key = user.anonymity_revoker_pubkey::<DryRun>().await?;

    let call = prepare_deposit_call(
        scheduler_account,
        amount,
        token,
        anonymity_revoker_public_key,
        user.address(),
        protocol_fee,
        memo,
    )?;

    let (tx_hash, block_hash) = match token {
        Token::Native => {
            user.new_account_native::<Call>(call.try_into().unwrap(), amount)
                .await?
        }
        Token::ERC20(address) => {
            user.approve_erc20::<Call>(address, app_state.contract_address, U256::MAX)
                .await?;
            user.new_account_erc20::<Call>(call.try_into().unwrap())
                .await?
        }
    };

    let new_account_event = get_event::<NewAccount>(
        &app_state.create_simple_provider().await?,
        tx_hash,
        block_hash,
    )
    .await?;
    debug!("New account event: {new_account_event:?}");

    scheduler_account.register_action(ShielderAction::new_account(
        amount,
        new_account_event.newNoteIndex,
        tx_hash,
        token,
        protocol_fee,
    ));
    info!("Created new account with {amount} tokens");
    Ok(new_account_event.newNoteIndex)
}

#[allow(clippy::too_many_arguments)]
async fn prepare_withdrawal_payload(
    app_state: &AppState,
    scheduler_account: &ShielderAccount,
    amount: U256,
    protocol_fee: U256,
    to: Address,
    token: Token,
    pocket_money: U256,
    memo: Vec<u8>,
    last_note_index: U256,
    max_relayer_fee: U256,
) -> Result<serde_json::Value> {
    // Get chain ID and contract version
    let provider = app_state.create_simple_provider().await?;
    let chain_id = provider.get_chain_id().await?;
    let contract_version = contract_version();

    // Convert contract version to bytes array - using note_version for now
    let version_bytes = [
        contract_version.note_version,
        contract_version.circuit_version,
        contract_version.patch_version,
    ];

    // Build payload
    let payload = serde_json::json!({
        "account_id": scheduler_account.id,
        "account_old_balance": scheduler_account.shielded_amount,
        "nullifier_old": scheduler_account.previous_nullifier(),
        "nullifier_new": scheduler_account.next_nullifier(),
        "last_note_index": last_note_index,
        "mac_salt": get_mac_salt(),
        "contract_version": format!("0x{}", hex::encode(version_bytes)),
        "chain_id": chain_id,
        "token_address": token.address(),
        "withdrawal_value": amount,
        "pocket_money": pocket_money,
        "withdraw_address": to,
        "protocol_fee": protocol_fee,
        "memo": format!("0x{}", hex::encode(memo)),
        "max_relayer_fee": max_relayer_fee
    });

    Ok(payload)
}

async fn get_tee_public_key(app_state: &AppState) -> Result<Vec<u8>> {
    let url = format!("{}/public_key", app_state.scheduler_url);
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to get TEE public key: {}",
            response.status()
        ));
    }

    let tee_response: TeePublicKeyResponse = response.json().await?;
    let public_key_bytes = BASE64.decode(&tee_response.tee_public_key.public_key)?;

    warn!(
        "Not verifying attestation document from scheduler TEE. Attestation document length: {}",
        tee_response.tee_public_key.attestation_document.len()
    );
    // TODO: verify attestation document

    Ok(public_key_bytes)
}

fn encrypt_payload(
    payload: &serde_json::Value,
    tee_public_key: &[u8],
) -> Result<EncryptionEnvelope> {
    // Step 1: Generate a random 256-bit Data Encryption Key (DEK) for AES-GCM
    let mut dek = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut dek);

    // Step 2: Encrypt the payload using AES-256-GCM with the DEK
    let mut iv = [0u8; 12]; // 12 bytes (96 bits) for GCM
    rand::thread_rng().fill_bytes(&mut iv);

    let cipher = Aes256Gcm::new_from_slice(&dek)
        .map_err(|e| anyhow!("Failed to create AES cipher: {}", e))?;

    let payload_bytes = serde_json::to_vec(payload)?;
    let nonce = Nonce::from_slice(&iv);

    let mut encrypted_payload = cipher
        .encrypt(nonce, payload_bytes.as_ref())
        .map_err(|e| anyhow!("Failed to encrypt payload: {}", e))?;

    // Extract auth tag (last 16 bytes)
    let auth_tag = encrypted_payload.split_off(encrypted_payload.len() - 16);

    // Step 3: Encrypt the DEK using RSA-OAEP with the TEE's public key
    let public_key = RsaPublicKey::from_public_key_der(tee_public_key)
        .map_err(|e| anyhow!("Failed to parse TEE public key: {}", e))?;

    let oaep_padding = Oaep::new::<Sha256>();
    let encrypted_dek = public_key
        .encrypt(&mut rand::thread_rng(), oaep_padding, &dek)
        .map_err(|e| anyhow!("Failed to encrypt DEK: {}", e))?;

    Ok(EncryptionEnvelope {
        encrypted_payload,
        encrypted_dek,
        iv: iv.to_vec(),
        auth_tag,
    })
}

async fn send_schedule_request(
    app_state: &AppState,
    encryption_envelope: EncryptionEnvelope,
    last_note_index: U256,
    pocket_money: u128,
    token_address: Address,
    relay_after: i64,
) -> Result<()> {
    let url = format!("{}/schedule_withdraw", app_state.scheduler_url);

    let request = ScheduleWithdrawRequest {
        encryption_envelope,
        last_note_index,
        relay_after,
        pocket_money: U256::from(pocket_money),
        token_address,
    };

    let client = reqwest::Client::new();
    let response = client.post(&url).json(&request).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow!(
            "Failed to schedule withdrawal: {} - {}",
            status,
            error_text
        ));
    }

    let schedule_response: ScheduleWithdrawResponse = response.json().await?;
    info!(
        "Scheduler response request id: {}\nmessage: {}",
        schedule_response.request_id, schedule_response.message
    );

    Ok(())
}

fn prepare_deposit_call(
    shielder_account: &ShielderAccount,
    amount: U256,
    token: Token,
    anonymity_revoker_public_key: GrumpkinPointAffine<U256>,
    caller_address: Address,
    protocol_fee: U256,
    memo: Bytes,
) -> Result<NewAccountCall> {
    let (params, pk) = get_proving_equipment(CircuitType::NewAccount)?;
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
