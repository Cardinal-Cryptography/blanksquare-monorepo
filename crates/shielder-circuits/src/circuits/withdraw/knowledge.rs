use halo2_proofs::halo2curves::ff::PrimeField;
use macros::embeddable;
use rand_core::RngCore;

use crate::{
    chips::viewing_key,
    consts::{
        merkle_constants::{ARITY, NOTE_TREE_HEIGHT},
        MAX_ACCOUNT_BALANCE_PASSING_RANGE_CHECK,
    },
    curve_arithmetic,
    embed::Embed,
    merkle::generate_example_path_with_given_leaf,
    note_hash,
    poseidon::off_circuit::hash,
    version::NOTE_VERSION,
    withdraw::{circuit::WithdrawCircuit, WithdrawInstance},
    Field, Fr, Note, ProverKnowledge, PublicInputProvider, Value,
};

#[derive(Clone, Debug, Default)]
#[embeddable(
    receiver = "WithdrawProverKnowledge<Value>",
    embedded = "WithdrawProverKnowledge<crate::AssignedCell>"
)]
pub struct WithdrawProverKnowledge<T> {
    pub withdrawal_value: T,

    // Additional public parameters that need to be included in proof
    pub commitment: T,

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
}

impl ProverKnowledge for WithdrawProverKnowledge<Fr> {
    type Circuit = WithdrawCircuit;
    type PublicInput = WithdrawInstance;

    /// TODO: Refactor this test. Having `MAX_ACCOUNT_BALANCE_PASSING_RANGE_CHECK` as the only
    /// non-random, non-trivial value is inconsistent with the function name and easy to overlook.
    /// Consider moving it to a separate test. Also evaluate removing randomness completely, as
    /// random values in unit tests are generally discouraged.
    ///
    /// All initial values are random, except for the account balances, the withdrawal value,
    /// and the relayer fee.
    ///
    /// `account_old_balance` has the largest possible value that passes the range check.
    fn random_correct_example(rng: &mut impl RngCore) -> Self {
        let id = curve_arithmetic::generate_user_id(Fr::random(&mut *rng).to_bytes());
        let nullifier_old = Fr::random(&mut *rng);

        let account_old_balance = Fr::from_u128(MAX_ACCOUNT_BALANCE_PASSING_RANGE_CHECK);
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
            withdrawal_value: Fr::ONE,
            commitment: Fr::random(&mut *rng),
            id,
            nullifier_old,
            account_old_balance,
            token_address,
            path,
            nullifier_new: Fr::random(&mut *rng),
            mac_salt: Fr::random(rng),
        }
    }

    fn create_circuit(&self) -> Self::Circuit {
        WithdrawCircuit(WithdrawProverKnowledge {
            nullifier_new: Value::known(self.nullifier_new),
            nullifier_old: Value::known(self.nullifier_old),

            account_old_balance: Value::known(self.account_old_balance),
            token_address: Value::known(self.token_address),

            id: Value::known(self.id),

            path: self.path.map(|level| level.map(Value::known)),

            withdrawal_value: Value::known(self.withdrawal_value),
            commitment: Value::known(self.commitment),
            mac_salt: Value::known(self.mac_salt),
        })
    }
}

impl PublicInputProvider<WithdrawInstance> for WithdrawProverKnowledge<Fr> {
    fn compute_public_input(&self, instance_id: WithdrawInstance) -> Fr {
        let viewing_key = viewing_key::off_circuit::derive_viewing_key(self.id);

        match instance_id {
            WithdrawInstance::MerkleRoot => hash(&self.path[NOTE_TREE_HEIGHT - 1]),
            WithdrawInstance::HashedOldNullifier => hash(&[self.nullifier_old]),
            WithdrawInstance::HashedNewNote => note_hash(&Note {
                version: NOTE_VERSION,
                id: self.id,
                nullifier: self.nullifier_new,
                account_balance: self.account_old_balance - self.withdrawal_value,
                token_address: self.token_address,
            }),
            WithdrawInstance::WithdrawalValue => self.withdrawal_value,
            WithdrawInstance::Commitment => self.commitment,
            WithdrawInstance::TokenAddress => self.token_address,
            WithdrawInstance::MacSalt => self.mac_salt,
            WithdrawInstance::MacCommitment => hash(&[self.mac_salt, viewing_key]),
        }
    }
}
