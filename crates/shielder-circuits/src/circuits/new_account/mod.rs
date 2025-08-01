use strum_macros::{EnumCount, EnumIter};

mod chip;
mod circuit;
mod knowledge;

pub use circuit::NewAccountCircuit;
pub use knowledge::NewAccountProverKnowledge;

use crate::chips::{mac::MacInstance, note::NoteInstance};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, EnumIter, EnumCount)]
pub enum NewAccountInstance {
    HashedNote,
    Prenullifier,
    InitialDeposit,
    Commitment,
    TokenAddress,
    AnonymityRevokerPublicKeyX,
    AnonymityRevokerPublicKeyY,
    EncryptedKeyCiphertext1X,
    EncryptedKeyCiphertext1Y,
    EncryptedKeyCiphertext2X,
    EncryptedKeyCiphertext2Y,
    MacSalt,
    MacCommitment,
}

impl TryFrom<NewAccountInstance> for NoteInstance {
    type Error = ();

    fn try_from(value: NewAccountInstance) -> Result<Self, Self::Error> {
        match value {
            NewAccountInstance::TokenAddress => Ok(NoteInstance::TokenAddress),
            _ => Err(()),
        }
    }
}

impl TryFrom<NewAccountInstance> for MacInstance {
    type Error = ();

    fn try_from(value: NewAccountInstance) -> Result<Self, Self::Error> {
        match value {
            NewAccountInstance::MacSalt => Ok(Self::MacSalt),
            NewAccountInstance::MacCommitment => Ok(Self::MacCommitment),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{vec, vec::Vec};

    use strum::IntoEnumIterator;

    use super::{NewAccountInstance, NewAccountInstance::*};

    #[test]
    fn instance_order() {
        // This is the order used in other parts of the codebase (e.g., in contracts).
        let expected_order = vec![
            HashedNote,
            Prenullifier,
            InitialDeposit,
            Commitment,
            TokenAddress,
            AnonymityRevokerPublicKeyX,
            AnonymityRevokerPublicKeyY,
            EncryptedKeyCiphertext1X,
            EncryptedKeyCiphertext1Y,
            EncryptedKeyCiphertext2X,
            EncryptedKeyCiphertext2Y,
            MacSalt,
            MacCommitment,
        ];
        assert_eq!(
            expected_order,
            NewAccountInstance::iter().collect::<Vec<_>>()
        );
    }
}
