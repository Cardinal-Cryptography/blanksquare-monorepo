use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use serde::{Deserialize, Serialize};
pub use shielder_relayer::RelayCalldata;
use shielder_setup::consts::{ARITY, TREE_HEIGHT};
pub use tokio_vsock::VMADDR_CID_ANY;

use crate::{
    base64_serialization,
    vsock::{VsockClient, VsockServer},
};
pub const VSOCK_PORT: u32 = 5000;

/// Payload for the `PrepareRelayCalldata` request.
/// The payload is encrypted using the TEE Public Key.
/// The TEE Public Key can be retrieved using the `TeePublicKey` request.
#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    pub account_id: U256,
    pub account_old_balance: U256,
    pub nullifier_old: U256,
    pub nullifier_new: U256,
    pub last_note_index: U256,

    pub mac_salt: U256,
    pub contract_version: FixedBytes<3>,
    pub chain_id: U256,
    pub token_address: Address,
    /// Total amount to be withdrawn. This amount includes fees (protocol and relayer fees).
    pub withdrawal_value: U256,
    pub withdraw_address: Address,
    pub pocket_money: U256,
    pub protocol_fee: U256,
    pub memo: Bytes,

    /// Maximum relayer fee.
    pub max_relayer_fee: U256,
    /// Timestamp after which the transaction can be relayed to chain.
    pub relay_after: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Message used to determine if TEE server is healthy
    Ping,

    /// Retrieves TEE Public Key, ie key which is used by the user to encrypt inputs to a circuit
    TeePublicKey,

    /// Request to prepare calldata for a relay transaction.
    PrepareRelayCalldata {
        /// User generated symetric key used to encrypt the payload
        #[serde(with = "base64_serialization")]
        sealed_data_encryption_key: Vec<u8>,
        /// Encrypted payload, see [`Payload`].
        /// It is encrypted using TEE Public Key.
        #[serde(with = "base64_serialization")]
        payload: Vec<u8>,
        /// Relayer fee
        relayer_fee: U256,
        /// Address of the relayer which will receive the relayer fee
        relayer_address: Address,
        /// Current merkle path
        merkle_path: Box<[[U256; ARITY]; TREE_HEIGHT]>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// Response to a health check,
    Pong,

    /// TEE Server public key, used to encrypt payload sent in [`Request::PrepareRelayCalldata`]
    TeePublicKey {
        #[serde(with = "base64_serialization")]
        public_key: Vec<u8>,
        #[serde(with = "base64_serialization")]
        attestation_document: Vec<u8>,
    },

    /// Response to [`Request::PrepareRelayCalldata`].
    /// Contains calldata that can be used to relay the transaction to the chain.
    PrepareRelayCalldata { calldata: RelayCalldata },
}

pub type TEEServer = VsockServer<Request, Response>;
pub type TEEClient = VsockClient<Request, Response>;
