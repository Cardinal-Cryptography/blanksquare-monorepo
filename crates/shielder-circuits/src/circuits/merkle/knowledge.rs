use core::borrow::Borrow;

use macros::embeddable;
use rand_core::RngCore;

use crate::{
    consts::merkle_constants::ARITY,
    embed::Embed,
    merkle::{circuit::MerkleCircuit, MerkleInstance},
    poseidon::off_circuit::hash,
    Field, Fr, ProverKnowledge, PublicInputProvider, Value,
};

#[derive(Clone, Debug)]
#[embeddable(
    receiver = "MerkleProverKnowledge<TREE_HEIGHT, Value>",
    impl_generics = "<const TREE_HEIGHT: usize>",
    embedded = "MerkleProverKnowledge<TREE_HEIGHT, crate::AssignedCell>"
)]
pub struct MerkleProverKnowledge<const TREE_HEIGHT: usize, T> {
    pub leaf: T,
    pub path: [[T; ARITY]; TREE_HEIGHT],
}

impl<const TREE_HEIGHT: usize, T: Default + Copy> Default
    for MerkleProverKnowledge<TREE_HEIGHT, T>
{
    fn default() -> Self {
        Self {
            leaf: T::default(),
            path: [[T::default(); ARITY]; TREE_HEIGHT],
        }
    }
}

impl<const TREE_HEIGHT: usize, T: Clone> MerkleProverKnowledge<TREE_HEIGHT, T> {
    pub fn new(leaf: impl Borrow<T>, path: impl Borrow<[[T; ARITY]; TREE_HEIGHT]>) -> Self {
        Self {
            leaf: leaf.borrow().clone(),
            path: path.borrow().clone(),
        }
    }
}

impl<const TREE_HEIGHT: usize> ProverKnowledge for MerkleProverKnowledge<TREE_HEIGHT, Fr> {
    type Circuit = MerkleCircuit<TREE_HEIGHT>;
    type PublicInput = MerkleInstance;

    fn random_correct_example(rng: &mut impl RngCore) -> Self {
        let mut path = [(); TREE_HEIGHT].map(|_| [(); ARITY].map(|_| Fr::random(&mut *rng)));
        for i in 1..TREE_HEIGHT {
            path[i][0] = hash(&path[i - 1]);
        }
        MerkleProverKnowledge::new(path[0][0], path)
    }

    fn create_circuit(&self) -> MerkleCircuit<TREE_HEIGHT> {
        MerkleCircuit(MerkleProverKnowledge {
            leaf: Value::known(self.leaf),
            path: self.path.map(|level| level.map(Value::known)),
        })
    }
}

impl<const TREE_HEIGHT: usize> PublicInputProvider<MerkleInstance>
    for MerkleProverKnowledge<TREE_HEIGHT, Fr>
{
    fn compute_public_input(&self, instance_id: MerkleInstance) -> Fr {
        match instance_id {
            MerkleInstance::MerkleRoot => hash(&self.path[TREE_HEIGHT - 1]),
        }
    }
}
