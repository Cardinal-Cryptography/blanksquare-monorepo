use rand_core::RngCore;
use strum_macros::{EnumCount, EnumIter};

use crate::{consts::merkle_constants::ARITY, poseidon::off_circuit::hash, Field, Fr};

mod chip;
mod circuit;
mod knowledge;

pub use chip::MerkleChip;
pub use circuit::MerkleCircuit;
pub use knowledge::MerkleProverKnowledge;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, EnumIter, EnumCount)]
pub enum MerkleInstance {
    MerkleRoot,
}

pub fn generate_example_path_with_given_leaf<const TREE_HEIGHT: usize>(
    leaf: Fr,
    rng: &mut impl RngCore,
) -> (Fr, [[Fr; ARITY]; TREE_HEIGHT]) {
    let mut path: [[Fr; ARITY]; TREE_HEIGHT] =
        [(); TREE_HEIGHT].map(|_| [(); ARITY].map(|_| Fr::random(&mut *rng)));
    path[0][0] = leaf;

    for i in 1..TREE_HEIGHT {
        path[i][(rng.next_u32() % (ARITY as u32)) as usize] = hash(&path[i - 1]);
    }

    let root = hash(&path[TREE_HEIGHT - 1]);

    (root, path)
}

#[cfg(test)]
mod tests {
    use std::{vec, vec::Vec};

    use strum::IntoEnumIterator;

    use super::{MerkleInstance, MerkleInstance::*};

    #[test]
    fn instance_order() {
        // This is the order used in other parts of the codebase (e.g., in contracts).
        let expected_order = vec![MerkleRoot];
        assert_eq!(expected_order, MerkleInstance::iter().collect::<Vec<_>>());
    }
}
