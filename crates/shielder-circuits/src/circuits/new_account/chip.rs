use halo2_proofs::{arithmetic::Field, halo2curves::bn256::Fr, plonk::Error};

use crate::{
    chips::{
        el_gamal::{ElGamalEncryptionChip, ElGamalEncryptionChipOutput, ElGamalEncryptionInput},
        mac::{MacChip, MacInput},
        note::{Note, NoteChip},
        to_affine::ToAffineChip,
        to_projective::ToProjectiveChip,
        viewing_key::ViewingKeyChip,
    },
    circuits::new_account::knowledge::NewAccountProverKnowledge,
    curve_arithmetic::{self, GrumpkinPointAffine},
    embed::Embed,
    gates::{is_point_on_curve_affine::IsPointOnCurveAffineGate, Gate},
    instance_wrapper::InstanceWrapper,
    new_account::NewAccountInstance::{self, *},
    poseidon::circuit::{hash, PoseidonChip},
    synthesizer::Synthesizer,
    version::NOTE_VERSION,
    AssignedCell, GrumpkinPoint,
};

#[derive(Clone, Debug)]
pub struct NewAccountChip {
    pub public_inputs: InstanceWrapper<NewAccountInstance>,
    pub poseidon: PoseidonChip,
    pub note: NoteChip,
    pub is_point_on_curve: IsPointOnCurveAffineGate,
    pub el_gamal_encryption: ElGamalEncryptionChip,
    pub to_projective: ToProjectiveChip,
    pub to_affine: ToAffineChip,
}

impl NewAccountChip {
    pub fn check_note(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &NewAccountProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        let note = self.note.note_hash(
            synthesizer,
            &Note {
                version: NOTE_VERSION,
                id: knowledge.id.clone(),
                nullifier: knowledge.nullifier.clone(),
                account_balance: knowledge.initial_deposit.clone(),
                token_address: knowledge.token_address.clone(),
            },
        )?;

        self.public_inputs.constrain_cells(
            synthesizer,
            [
                (note, HashedNote),
                (knowledge.initial_deposit.clone(), InitialDeposit),
            ],
        )
    }

    pub fn constrain_prenullifier(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &NewAccountProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        let h_id = hash(synthesizer, self.poseidon.clone(), [knowledge.id.clone()])?;
        self.public_inputs
            .constrain_cells(synthesizer, [(h_id, Prenullifier)])
    }

    /// assert that `key` is an x-coordinate of a point on the Grumpkin curve, i.e.,
    /// y^2 = key^3 - 17, for some y, if yes, outputs one such y (out of two possible)
    fn constrain_viewing_key_encodable(
        &self,
        synthesizer: &mut impl Synthesizer,
        key: AssignedCell,
    ) -> Result<AssignedCell, Error> {
        let y_squared_value =
            curve_arithmetic::quadratic_residue_given_x_affine(key.value().copied());
        let y_value =
            y_squared_value.map(|v| v.sqrt().expect("element does not have a square root"));
        let y = y_value.embed(synthesizer, "y")?;

        self.is_point_on_curve
            .apply_in_new_region(synthesizer, GrumpkinPointAffine::new(key, y.clone()))?;
        Ok(y)
    }

    pub fn constrain_encrypting_viewing_key(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &NewAccountProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        let viewing_key = ViewingKeyChip::new(self.poseidon.clone())
            .derive_viewing_key(synthesizer, knowledge.id.clone())?;

        let y = self.constrain_viewing_key_encodable(synthesizer, viewing_key.clone())?;

        let revoker_pkey = knowledge.anonymity_revoker_public_key.clone();

        let revoker_pkey_projective = self
            .to_projective
            .to_projective(synthesizer, &revoker_pkey)?;

        let z = synthesizer.assign_constant("ONE", Fr::ONE)?;

        let ElGamalEncryptionChipOutput {
            ciphertext1: c1_projective,
            ciphertext2: c2_projective,
        } = self.el_gamal_encryption.encrypt(
            synthesizer,
            &ElGamalEncryptionInput {
                message: GrumpkinPoint::new(viewing_key, y, z),
                public_key: revoker_pkey_projective,
                salt_le_bits: knowledge.encryption_salt.clone(),
            },
        )?;

        let c1_affine = self.to_affine.to_affine(synthesizer, &c1_projective)?;
        let c2_affine = self.to_affine.to_affine(synthesizer, &c2_projective)?;

        self.public_inputs.constrain_cells(
            synthesizer,
            [
                (revoker_pkey.x, AnonymityRevokerPublicKeyX),
                (revoker_pkey.y, AnonymityRevokerPublicKeyY),
                (c1_affine.x, EncryptedKeyCiphertext1X),
                (c1_affine.y, EncryptedKeyCiphertext1Y),
                (c2_affine.x, EncryptedKeyCiphertext2X),
                (c2_affine.y, EncryptedKeyCiphertext2Y),
            ],
        )
    }

    pub fn check_mac(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &NewAccountProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        let viewing_key = ViewingKeyChip::new(self.poseidon.clone())
            .derive_viewing_key(synthesizer, knowledge.id.clone())?;

        MacChip::new(self.poseidon.clone(), self.public_inputs.narrow()).mac(
            synthesizer,
            &MacInput {
                key: viewing_key,
                salt: knowledge.mac_salt.clone(),
            },
        )?;

        Ok(())
    }

    pub fn check_commitment(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &NewAccountProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        self.public_inputs
            .constrain_cells(synthesizer, [(knowledge.commitment.clone(), Commitment)])
    }
}
