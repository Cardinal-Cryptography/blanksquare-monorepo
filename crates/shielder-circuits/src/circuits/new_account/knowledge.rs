use halo2_proofs::halo2curves::grumpkin;
use macros::embeddable;
use rand_core::RngCore;

use crate::{
    chips::{
        el_gamal::{self},
        viewing_key,
    },
    consts::FIELD_BITS,
    curve_arithmetic::{self, GrumpkinPointAffine},
    embed::Embed,
    field_element_to_le_bits, le_bits_to_field_element,
    new_account::{circuit::NewAccountCircuit, NewAccountInstance},
    note_hash,
    poseidon::off_circuit::hash,
    version::NOTE_VERSION,
    Field, Fr, Note, ProverKnowledge, PublicInputProvider, Value,
};

#[derive(Clone, Debug)]
#[embeddable(
    receiver = "NewAccountProverKnowledge<Value>",
    embedded = "NewAccountProverKnowledge<crate::AssignedCell>"
)]
pub struct NewAccountProverKnowledge<T> {
    pub id: T,
    pub nullifier: T,
    pub initial_deposit: T,
    pub commitment: T,
    pub token_address: T,
    pub encryption_salt: [T; FIELD_BITS],
    pub anonymity_revoker_public_key: GrumpkinPointAffine<T>,
    pub mac_salt: T,
}

impl<T: Default + Copy> Default for NewAccountProverKnowledge<T> {
    fn default() -> Self {
        Self {
            id: T::default(),
            nullifier: T::default(),
            initial_deposit: T::default(),
            commitment: T::default(),
            token_address: T::default(),
            encryption_salt: [T::default(); FIELD_BITS],
            anonymity_revoker_public_key: GrumpkinPointAffine::default(),
            mac_salt: T::default(),
        }
    }
}

impl ProverKnowledge for NewAccountProverKnowledge<Fr> {
    type Circuit = NewAccountCircuit;
    type PublicInput = NewAccountInstance;

    fn random_correct_example(rng: &mut impl RngCore) -> Self {
        Self {
            id: curve_arithmetic::generate_user_id(Fr::random(&mut *rng).to_bytes()),
            nullifier: Fr::random(&mut *rng),
            initial_deposit: Fr::ONE,
            commitment: Fr::random(&mut *rng),
            token_address: Fr::ZERO,
            encryption_salt: field_element_to_le_bits(grumpkin::Fr::ONE),
            anonymity_revoker_public_key: GrumpkinPointAffine::random(rng),
            mac_salt: Fr::random(rng),
        }
    }

    fn create_circuit(&self) -> Self::Circuit {
        NewAccountCircuit(NewAccountProverKnowledge {
            id: Value::known(self.id),
            nullifier: Value::known(self.nullifier),
            initial_deposit: Value::known(self.initial_deposit),
            commitment: Value::known(self.commitment),
            token_address: Value::known(self.token_address),
            encryption_salt: self.encryption_salt.map(Value::known),
            anonymity_revoker_public_key: GrumpkinPointAffine::new(
                Value::known(self.anonymity_revoker_public_key.x),
                Value::known(self.anonymity_revoker_public_key.y),
            ),
            mac_salt: Value::known(self.mac_salt),
        })
    }
}

impl PublicInputProvider<NewAccountInstance> for NewAccountProverKnowledge<Fr> {
    fn compute_public_input(&self, instance_id: NewAccountInstance) -> Fr {
        let viewing_key = viewing_key::off_circuit::derive_viewing_key(self.id);
        let y = curve_arithmetic::quadratic_residue_given_x_affine(viewing_key)
            .sqrt()
            .expect("element has a square root");

        let salt: grumpkin::Fr = le_bits_to_field_element(&self.encryption_salt);

        let (c1, c2) = el_gamal::off_circuit::encrypt(
            GrumpkinPointAffine::new(viewing_key, y).into(),
            self.anonymity_revoker_public_key.into(),
            salt,
        );

        let ciphertext1: GrumpkinPointAffine<Fr> = c1.into();
        let ciphertext2: GrumpkinPointAffine<Fr> = c2.into();

        match instance_id {
            NewAccountInstance::HashedNote => note_hash(&Note {
                version: NOTE_VERSION,
                id: self.id,
                nullifier: self.nullifier,
                account_balance: self.initial_deposit,
                token_address: self.token_address,
            }),
            NewAccountInstance::Prenullifier => hash(&[self.id]),
            NewAccountInstance::InitialDeposit => self.initial_deposit,
            NewAccountInstance::Commitment => self.commitment,
            NewAccountInstance::TokenAddress => self.token_address,
            NewAccountInstance::AnonymityRevokerPublicKeyX => self.anonymity_revoker_public_key.x,
            NewAccountInstance::AnonymityRevokerPublicKeyY => self.anonymity_revoker_public_key.y,
            NewAccountInstance::EncryptedKeyCiphertext1X => ciphertext1.x,
            NewAccountInstance::EncryptedKeyCiphertext1Y => ciphertext1.y,
            NewAccountInstance::EncryptedKeyCiphertext2X => ciphertext2.x,
            NewAccountInstance::EncryptedKeyCiphertext2Y => ciphertext2.y,
            NewAccountInstance::MacSalt => self.mac_salt,
            NewAccountInstance::MacCommitment => hash(&[self.mac_salt, viewing_key]),
        }
    }
}
