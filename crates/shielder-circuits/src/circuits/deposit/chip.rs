use halo2_proofs::plonk::Error;

use crate::{
    chips::{
        mac::{MacChip, MacInput},
        note::{Note, NoteChip},
        viewing_key::ViewingKeyChip,
    },
    circuits::{
        deposit::knowledge::DepositProverKnowledge,
        merkle::{MerkleChip, MerkleProverKnowledge},
    },
    deposit::DepositInstance::{self, Commitment, DepositValue, HashedNewNote, HashedOldNullifier},
    instance_wrapper::InstanceWrapper,
    poseidon::circuit::{hash, PoseidonChip},
    synthesizer::Synthesizer,
    version::NOTE_VERSION,
    AssignedCell,
};

#[derive(Clone, Debug)]
pub struct DepositChip {
    pub public_inputs: InstanceWrapper<DepositInstance>,
    pub poseidon: PoseidonChip,
    pub merkle: MerkleChip,
    pub note: NoteChip,
}

impl DepositChip {
    pub fn check_old_note(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &DepositProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        let old_note = self.note.note_hash(
            synthesizer,
            &Note {
                version: NOTE_VERSION,
                id: knowledge.id.clone(),
                nullifier: knowledge.nullifier_old.clone(),
                account_balance: knowledge.account_old_balance.clone(),
                token_address: knowledge.token_address.clone(),
            },
        )?;

        self.merkle.synthesize(
            synthesizer,
            &MerkleProverKnowledge::new(old_note, &knowledge.path),
        )
    }

    pub fn check_old_nullifier(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &DepositProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        let hashed_old_nullifier = hash(
            synthesizer,
            self.poseidon.clone(),
            [knowledge.nullifier_old.clone()],
        )?;

        self.public_inputs
            .constrain_cells(synthesizer, [(hashed_old_nullifier, HashedOldNullifier)])
    }

    pub fn check_new_note(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &DepositProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        self.public_inputs.constrain_cells(
            synthesizer,
            [(knowledge.deposit_value.clone(), DepositValue)],
        )?;

        let account_balance_new = self.note.increase_balance(
            synthesizer,
            knowledge.account_old_balance.clone(),
            knowledge.deposit_value.clone(),
        )?;

        let new_note = self.note.note_hash(
            synthesizer,
            &Note {
                version: NOTE_VERSION,
                id: knowledge.id.clone(),
                nullifier: knowledge.nullifier_new.clone(),
                account_balance: account_balance_new,
                token_address: knowledge.token_address.clone(),
            },
        )?;

        self.public_inputs
            .constrain_cells(synthesizer, [(new_note, HashedNewNote)])
    }

    pub fn check_mac(
        &self,
        synthesizer: &mut impl Synthesizer,
        knowledge: &DepositProverKnowledge<AssignedCell>,
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
        knowledge: &DepositProverKnowledge<AssignedCell>,
    ) -> Result<(), Error> {
        self.public_inputs
            .constrain_cells(synthesizer, [(knowledge.commitment.clone(), Commitment)])
    }
}
