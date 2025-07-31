use strum_macros::{EnumCount, EnumIter};

use crate::{chips::note::NoteInstance, merkle::MerkleInstance};

mod chip;
mod circuit;
mod knowledge;

pub use circuit::WithdrawCircuit;
pub use knowledge::WithdrawProverKnowledge;

use crate::chips::mac::MacInstance;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, EnumIter, EnumCount)]
pub enum WithdrawInstance {
    MerkleRoot,
    HashedOldNullifier,
    HashedNewNote,
    WithdrawalValue,
    TokenAddress,
    Commitment,
    MacSalt,
    MacCommitment,
}

impl TryFrom<WithdrawInstance> for MerkleInstance {
    type Error = ();

    fn try_from(value: WithdrawInstance) -> Result<Self, Self::Error> {
        match value {
            WithdrawInstance::MerkleRoot => Ok(Self::MerkleRoot),
            _ => Err(()),
        }
    }
}

impl TryFrom<WithdrawInstance> for NoteInstance {
    type Error = ();

    fn try_from(value: WithdrawInstance) -> Result<Self, Self::Error> {
        match value {
            WithdrawInstance::TokenAddress => Ok(NoteInstance::TokenAddress),
            _ => Err(()),
        }
    }
}

impl TryFrom<WithdrawInstance> for MacInstance {
    type Error = ();

    fn try_from(value: WithdrawInstance) -> Result<Self, Self::Error> {
        match value {
            WithdrawInstance::MacSalt => Ok(Self::MacSalt),
            WithdrawInstance::MacCommitment => Ok(Self::MacCommitment),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{vec, vec::Vec};

    use strum::IntoEnumIterator;

    use super::{WithdrawInstance, WithdrawInstance::*};

    #[test]
    fn instance_order() {
        // This is the order used in other parts of the codebase (e.g., in contracts).
        let expected_order = vec![
            MerkleRoot,
            HashedOldNullifier,
            HashedNewNote,
            WithdrawalValue,
            TokenAddress,
            Commitment,
            MacSalt,
            MacCommitment,
        ];
        assert_eq!(expected_order, WithdrawInstance::iter().collect::<Vec<_>>());
    }
}
