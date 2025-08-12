use serde::{Deserialize, Serialize};

use crate::{
    base64_serialization,
    vsock::{VsockClient, VsockServer},
};

pub const VSOCK_PORT: u16 = 5000;

/// Request to generate proof. A `payload` is encrypted `ciphertext=(pub_sk, circuit_type, circuit_inputs)`, where
/// * `pub_sk` is a user public key, expressed as a vector of bytes, compatible with [ecies-encryption-lib](https://github.com/Cardinal-Cryptography/ecies-encryption-lib),
/// * `circuit_type` is a byte (u8), see [`CircuitType`]. This field is required to decode `circuit_inputs`
/// * `circuit_inputs` is a (`R`, `w`, `s`) - a relation, witness and statement of ZK-proof we want to generate, under the hood, this is a JSON object, byte-encoded (UTF-8)
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestGenerateProofPayload {
    pub circuit_type: CircuitType,
    #[serde(with = "base64_serialization")]
    pub user_public_key: Vec<u8>,
    #[serde(with = "base64_serialization")]
    pub circuit_inputs: Vec<u8>,
}

/// Response to a request for generating proof. Contains proof and public inputs.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseGenerateProofPayload {
    #[serde(with = "base64_serialization")]
    pub proof: Vec<u8>,
    #[serde(with = "base64_serialization")]
    pub pub_inputs: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Message used to determine if TEE server is healthy
    Ping,

    /// Retrieves TEE Public Key, ie key which is used by the user to encrypt inputs to a circuit
    TeePublicKey,

    /// Request for generate proof and pub inputs.
    /// It is encrypted using TEE Public Key.
    /// For `payload` schema, see [`RequestGenerateProofPayload`].
    GenerateProof {
        #[serde(with = "base64_serialization")]
        payload: Vec<u8>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// Response to a health check,
    Pong,

    /// TEE Server public key, used to encrypt payload sent in [`Request::GenerateProof`]
    TeePublicKey {
        public_key: String,
        #[serde(with = "base64_serialization")]
        attestation_document: Vec<u8>,
    },

    /// A ZK-proof computed on the [`Request::GenerateProof`] request.
    /// It is encrypted using a public key sent in the request.
    /// For `payload` schema, see [`ResponseGenerateProofPayload`].
    EncryptedProof {
        //    payload: String,
        #[serde(with = "base64_serialization")]
        payload: Vec<u8>,
    },
}

pub type ProverServer = VsockServer<Request, Response>;
pub type ProverClient = VsockClient<Request, Response>;

#[derive(Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum CircuitType {
    NewAccount,
    Deposit,
    Withdraw,
}
