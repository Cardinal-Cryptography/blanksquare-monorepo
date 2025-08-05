use macros::embeddable;
use rand_core::RngCore;

use crate::{
    chips::viewing_key,
    consts::merkle_constants::{ARITY, NOTE_TREE_HEIGHT},
    curve_arithmetic,
    deposit::{circuit::DepositCircuit, DepositInstance},
    embed::Embed,
    merkle::generate_example_path_with_given_leaf,
    note_hash,
    poseidon::off_circuit::hash,
    version::NOTE_VERSION,
    Field, Fr, Note, ProverKnowledge, PublicInputProvider, Value,
};

/// Stores values needed to compute example inputs for `DepositCircuit`. Provides a function
/// to create such inputs.
///
/// Some of the fields of this struct are private inputs, some are public inputs,
/// and some do not appear as inputs at all, but are just intermediate advice values.
#[derive(Clone, Debug, Default)]
#[embeddable(
    receiver = "DepositProverKnowledge<Value>",
    embedded = "DepositProverKnowledge<crate::AssignedCell>"
)]
pub struct DepositProverKnowledge<T> {
    // Old note
    pub id: T,
    pub nullifier_old: T,
    pub account_old_balance: T,
    pub token_address: T,

    // Merkle proof
    pub path: [[T; ARITY]; NOTE_TREE_HEIGHT],

    // New note
    pub nullifier_new: T,

    // Salt for MAC.
    pub mac_salt: T,

    pub deposit_value: T,
    pub commitment: T,
}

impl ProverKnowledge for DepositProverKnowledge<Fr> {
    type Circuit = DepositCircuit;
    type PublicInput = DepositInstance;

    /// Creates a random example with correct inputs. All values are random except for the deposit
    /// amount and the old account balances.
    fn random_correct_example(rng: &mut impl RngCore) -> Self {
        let id = curve_arithmetic::generate_user_id(Fr::random(&mut *rng).to_bytes());

        let nullifier_old = Fr::random(&mut *rng);
        let account_old_balance = Fr::from(10);
        let token_address = Fr::ZERO;
        let h_note_old = note_hash(&Note {
            version: NOTE_VERSION,
            id,
            nullifier: nullifier_old,
            account_balance: account_old_balance,
            token_address,
        });
        let (_, path) = generate_example_path_with_given_leaf(h_note_old, &mut *rng);
        Self {
            id,
            nullifier_old,
            account_old_balance,
            token_address,
            path,
            nullifier_new: Fr::random(&mut *rng),
            deposit_value: Fr::ONE,
            mac_salt: Fr::random(&mut *rng),
            commitment: Fr::random(rng),
        }
    }

    fn create_circuit(&self) -> Self::Circuit {
        DepositCircuit(DepositProverKnowledge {
            nullifier_new: Value::known(self.nullifier_new),
            nullifier_old: Value::known(self.nullifier_old),
            account_old_balance: Value::known(self.account_old_balance),
            token_address: Value::known(self.token_address),
            id: Value::known(self.id),
            path: self.path.map(|level| level.map(Value::known)),
            deposit_value: Value::known(self.deposit_value),
            mac_salt: Value::known(self.mac_salt),
            commitment: Value::known(self.commitment),
        })
    }
}

impl PublicInputProvider<DepositInstance> for DepositProverKnowledge<Fr> {
    fn compute_public_input(&self, instance_id: DepositInstance) -> Fr {
        let viewing_key = viewing_key::off_circuit::derive_viewing_key(self.id);

        match instance_id {
            DepositInstance::MerkleRoot => hash(&self.path[NOTE_TREE_HEIGHT - 1]),
            DepositInstance::HashedOldNullifier => hash(&[self.nullifier_old]),
            DepositInstance::HashedNewNote => note_hash(&Note {
                version: NOTE_VERSION,
                id: self.id,
                nullifier: self.nullifier_new,
                account_balance: self.account_old_balance + self.deposit_value,
                token_address: self.token_address,
            }),
            DepositInstance::DepositValue => self.deposit_value,
            DepositInstance::Commitment => self.commitment,
            DepositInstance::TokenAddress => self.token_address,
            DepositInstance::MacSalt => self.mac_salt,
            DepositInstance::MacCommitment => hash(&[self.mac_salt, viewing_key]),
        }
    }
}
