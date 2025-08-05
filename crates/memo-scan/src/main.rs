use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use ecies_encryption_lib::{utils::from_hex, PrivKey};
use serde_json;

use crate::scan::scan_blocks;
pub mod scan;
mod utils;

/// Scan blocks for referrals in the shielder contract.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Contract address (hex string, e.g. 0x5A0dea46A96a5b578c9cf1730f461eD0bC9C32c6)
    #[arg(long)]
    contract_address: String,

    /// RPC URL
    #[arg(long)]
    rpc_url: String,

    /// Start block number
    #[arg(long)]
    start_block: Option<u64>,

    /// Stop block number
    #[arg(long)]
    stop_block: Option<u64>,

    /// Referral encryption private key
    #[arg(long)]
    referral_private_key_hex: Option<String>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    let contract_address = args
        .contract_address
        .parse::<Address>()
        .map_err(|_| anyhow::anyhow!("Invalid contract address"))?;

    let referral_private_key = if let Some(pk) = args.referral_private_key_hex {
        Some(PrivKey::from_bytes(&from_hex(&pk)?)?)
    } else {
        None
    };

    let referrals = scan_blocks(
        &args.rpc_url,
        &contract_address,
        args.start_block,
        args.stop_block,
        referral_private_key,
    )
    .await?;
    eprintln!("Found {} referrals", referrals.len());
    println!("{}", serde_json::to_string_pretty(&referrals)?);
    Ok(())
}
