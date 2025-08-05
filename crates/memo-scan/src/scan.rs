use alloy_network::AnyNetwork;
use alloy_primitives::{Address, Bytes};
use alloy_provider::Provider;
use alloy_rpc_types::{Filter, Log};
use alloy_sol_types::SolEvent;
use alloy_transport::BoxTransport;
use anyhow::Result;
use ecies_encryption_lib::{decrypt_padded_unchecked, PrivKey};
use shielder_contract::{
    providers::create_simple_provider,
    ShielderContract::{Deposit, NewAccount, Withdraw},
};

use crate::utils::get_contract_deployment_block_num;

const BATCH_LENGTH: usize = 10000;

#[derive(serde::Serialize, Debug)]
pub struct Referral {
    pub block_number: u64,
    pub transaction_hash: [u8; 32],
    pub memo: Bytes,
    pub referral_id: Option<String>,
}

pub async fn scan_blocks(
    rpc_url: &str,
    contract_address: &Address,
    start_block: Option<u64>,
    stop_block: Option<u64>,
    referral_private_key: Option<PrivKey>,
) -> Result<Vec<Referral>> {
    let provider = create_simple_provider(rpc_url).await?;
    let deployment_block_num =
        get_contract_deployment_block_num(&provider, contract_address).await?;

    let current_num = provider.get_block_number().await?;
    eprintln!("Contract {contract_address} was deployed at block number: {deployment_block_num}");
    eprintln!("Current block number: {current_num}");
    let start_block = match start_block {
        Some(num) => {
            if num < deployment_block_num {
                return Err(anyhow::anyhow!(
                    "Start block {} is before contract deployment at block {}",
                    num,
                    deployment_block_num
                ));
            }
            num
        }
        None => deployment_block_num,
    };
    let stop_block = match stop_block {
        Some(num) => {
            if num > current_num {
                return Err(anyhow::anyhow!(
                    "Stop block {} is after current block {}",
                    num,
                    current_num
                ));
            }
            num
        }
        None => current_num,
    };

    find_referrals(
        &provider,
        contract_address,
        start_block,
        stop_block,
        referral_private_key,
    )
    .await
}

async fn find_referrals(
    provider: &impl Provider<BoxTransport, AnyNetwork>,
    contract_address: &Address,
    start_block: u64,
    stop_block: u64,
    referral_private_key: Option<PrivKey>,
) -> Result<Vec<Referral>> {
    let mut referrals = Vec::new();
    let mut next_block_to_process = start_block;
    while next_block_to_process <= stop_block {
        let end_block = (next_block_to_process + BATCH_LENGTH as u64).min(stop_block);
        eprintln!(
            "Processing blocks from {} to {}",
            next_block_to_process, end_block
        );
        let filter = Filter::new()
            .address(*contract_address)
            .from_block(next_block_to_process)
            .to_block(end_block);

        let raw_logs = provider.get_logs(&filter).await?;

        for log in raw_logs {
            let referral = find_referral(log, referral_private_key.as_ref())?;
            if let Some(referral) = referral {
                referrals.push(referral);
            }
        }
        next_block_to_process = end_block + 1;
    }

    Ok(referrals)
}

fn find_referral(log: Log, referral_private_key: Option<&PrivKey>) -> Result<Option<Referral>> {
    let tx_hash: [u8; 32] = log
        .transaction_hash
        .ok_or(anyhow::anyhow!("Missing transaction hash"))?
        .0;
    let block_number = log
        .block_number
        .ok_or(anyhow::anyhow!("Missing block number"))?;
    let memo = match log.topic0() {
        Some(&NewAccount::SIGNATURE_HASH) => {
            let new_account = NewAccount::decode_log_data(log.data(), true)?;
            new_account.memo
        }
        Some(&Deposit::SIGNATURE_HASH) => {
            let deposit = Deposit::decode_log_data(log.data(), true)?;
            deposit.memo
        }
        Some(&Withdraw::SIGNATURE_HASH) => {
            let withdraw = Withdraw::decode_log_data(log.data(), true)?;
            withdraw.memo
        }
        _ => return Ok(None), // Not a relevant event
    };
    let referral_id = match (referral_private_key, memo.is_empty()) {
        (None, _) => {
            // If no private key is provided, we skip decryption
            None
        }
        (_, true) => {
            // If memo is empty, it means no referral ID was set
            eprintln!("Empty memo for transaction {}", Bytes::from(tx_hash));
            return Ok(None);
        }
        (Some(referral_private_key), false) => {
            // Attempt to decrypt the memo
            match decrypt_padded_unchecked(&memo, referral_private_key) {
                Ok(decrypted) => match String::from_utf8(decrypted) {
                    Ok(referral_id) => Some(referral_id),
                    Err(_) => {
                        eprintln!(
                            "Decrypted memo contains invalid UTF-8 for transaction {}",
                            Bytes::from(tx_hash)
                        );
                        None
                    }
                },
                Err(err) => {
                    eprintln!(
                        "Failed to decrypt memo for transaction {}\n Error: {}\n Either the memo is not encrypted or the private key is incorrect.",
                        Bytes::from(tx_hash),
                        err
                    );
                    None
                }
            }
        }
    };
    Ok(Some(Referral {
        block_number,
        transaction_hash: tx_hash,
        memo,
        referral_id,
    }))
}
