use alloc::vec::Vec;

use shielder_circuits::{
    deposit::{DepositInstance, DepositProverKnowledge},
    Fr, PublicInputProvider,
};
use type_conversions::field_to_bytes;
#[cfg(feature = "build-wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

use super::error::VerificationError;
use crate::utils::{vec_to_f, vec_to_path};

#[cfg_attr(feature = "build-uniffi", derive(uniffi::Record))]
// `getter_with_clone` is required for `Vec<u8>` struct fields
#[cfg_attr(feature = "build-wasm", wasm_bindgen(getter_with_clone))]
#[derive(Clone, Debug, Default)]
pub struct DepositPubInputsBytes {
    pub merkle_root: Vec<u8>,
    pub h_nullifier_old: Vec<u8>,
    pub h_note_new: Vec<u8>,
    pub value: Vec<u8>,
    pub commitment: Vec<u8>,
    pub token_address: Vec<u8>,
    pub mac_salt: Vec<u8>,
    pub mac_commitment: Vec<u8>,
}

impl From<DepositProverKnowledge<Fr>> for DepositPubInputsBytes {
    fn from(knowledge: DepositProverKnowledge<Fr>) -> Self {
        DepositPubInputsBytes {
            merkle_root: field_to_bytes(
                knowledge.compute_public_input(DepositInstance::MerkleRoot),
            ),
            h_nullifier_old: field_to_bytes(
                knowledge.compute_public_input(DepositInstance::HashedOldNullifier),
            ),
            h_note_new: field_to_bytes(
                knowledge.compute_public_input(DepositInstance::HashedNewNote),
            ),
            value: field_to_bytes(knowledge.compute_public_input(DepositInstance::DepositValue)),
            commitment: field_to_bytes(knowledge.compute_public_input(DepositInstance::Commitment)),
            token_address: field_to_bytes(
                knowledge.compute_public_input(DepositInstance::TokenAddress),
            ),
            mac_salt: field_to_bytes(knowledge.compute_public_input(DepositInstance::MacSalt)),
            mac_commitment: field_to_bytes(
                knowledge.compute_public_input(DepositInstance::MacCommitment),
            ),
        }
    }
}

#[cfg_attr(feature = "build-uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "build-wasm", wasm_bindgen)]
#[derive(Clone, Debug)]
pub struct DepositCircuit(super::DepositCircuit);

#[cfg(feature = "build-wasm")]
#[cfg_attr(feature = "build-wasm", wasm_bindgen)]
impl DepositCircuit {
    #[cfg_attr(feature = "build-wasm", wasm_bindgen(constructor))]
    pub fn new_pronto(params_buf: &[u8], pk_buf: &[u8]) -> Self {
        DepositCircuit(super::DepositCircuit::new_pronto(params_buf, pk_buf))
    }
}

#[cfg(not(feature = "build-wasm"))]
#[cfg_attr(feature = "build-uniffi", uniffi::export)]
impl DepositCircuit {
    #[cfg_attr(feature = "build-uniffi", uniffi::constructor)]
    pub fn new_pronto() -> Self {
        DepositCircuit(super::DepositCircuit::new_pronto(
            include_bytes!("../../artifacts/deposit/params.bin"),
            include_bytes!("../../artifacts/deposit/pk.bin"),
        ))
    }
}

#[cfg_attr(feature = "build-uniffi", uniffi::export)]
#[cfg_attr(feature = "build-wasm", wasm_bindgen)]
impl DepositCircuit {
    #[allow(clippy::too_many_arguments)]
    pub fn prove(
        &self,
        id: Vec<u8>,
        nullifier_old: Vec<u8>,
        account_balance_old: Vec<u8>,
        token_address: Vec<u8>,
        path: Vec<u8>,
        value: Vec<u8>,
        commitment: Vec<u8>,
        nullifier_new: Vec<u8>,
        mac_salt: Vec<u8>,
    ) -> Vec<u8> {
        self.0.prove(
            &DepositProverKnowledge {
                id: vec_to_f(id),
                nullifier_old: vec_to_f(nullifier_old),
                account_old_balance: vec_to_f(account_balance_old),
                token_address: vec_to_f(token_address),
                path: vec_to_path(path),
                deposit_value: vec_to_f(value),
                commitment: vec_to_f(commitment),
                nullifier_new: vec_to_f(nullifier_new),
                mac_salt: vec_to_f(mac_salt),
            },
            &mut rand::thread_rng(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        merkle_root: Vec<u8>,
        h_nullifier_old: Vec<u8>,
        h_note_new: Vec<u8>,
        value: Vec<u8>,
        commitment: Vec<u8>,
        token_address: Vec<u8>,
        mac_salt: Vec<u8>,
        mac_commitment: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(), VerificationError> {
        let public_input = |input: DepositInstance| {
            let value = match input {
                DepositInstance::MerkleRoot => &merkle_root,
                DepositInstance::HashedOldNullifier => &h_nullifier_old,
                DepositInstance::HashedNewNote => &h_note_new,
                DepositInstance::DepositValue => &value,
                DepositInstance::Commitment => &commitment,
                DepositInstance::TokenAddress => &token_address,
                DepositInstance::MacSalt => &mac_salt,
                DepositInstance::MacCommitment => &mac_commitment,
            };
            vec_to_f(value.clone())
        };

        self.0.verify(&public_input, proof).map_err(Into::into)
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(feature = "build-wasm", wasm_bindgen)]
#[cfg_attr(feature = "build-uniffi", uniffi::export)]
pub fn deposit_pub_inputs(
    id: Vec<u8>,
    nullifier_old: Vec<u8>,
    account_balance_old: Vec<u8>,
    token_address: Vec<u8>,
    path: Vec<u8>,
    value: Vec<u8>,
    commitment: Vec<u8>,
    nullifier_new: Vec<u8>,
    mac_salt: Vec<u8>,
) -> DepositPubInputsBytes {
    let knowledge = DepositProverKnowledge {
        id: vec_to_f(id),
        nullifier_old: vec_to_f(nullifier_old),
        account_old_balance: vec_to_f(account_balance_old),
        token_address: vec_to_f(token_address),
        path: vec_to_path(path),
        deposit_value: vec_to_f(value),
        commitment: vec_to_f(commitment),
        nullifier_new: vec_to_f(nullifier_new),
        mac_salt: vec_to_f(mac_salt),
    };

    knowledge.into()
}
