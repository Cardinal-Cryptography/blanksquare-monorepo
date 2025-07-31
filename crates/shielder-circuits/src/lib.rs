#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod chips;
pub mod circuits;
mod column_pool;
mod config_builder;
pub mod consts;
pub mod curve_arithmetic;
mod embed;
mod gates;
mod instance_wrapper;
pub mod poseidon;
mod range_table;
mod synthesizer;
mod version;

use alloc::{fmt::Debug, vec::Vec};

pub use chips::{
    el_gamal::off_circuit::{decrypt, encrypt, generate_keys},
    note::{off_circuit::note_hash, Note},
    viewing_key::off_circuit::derive_viewing_key,
};
pub use circuits::*;
pub use consts::MAX_K;
pub use curve_arithmetic::{grumpkin_point::*, *};
pub use halo2_proofs::{
    arithmetic::Field,
    dev::CircuitCost,
    halo2curves::{
        bn256::{Bn256, Fr, G1Affine, G1},
        ff::PrimeField,
        grumpkin,
    },
    plonk::{Circuit, ProvingKey, VerifyingKey},
    poly::{commitment::Params, kzg::commitment::ParamsKZG},
    SerdeFormat,
};
use rand_core::RngCore;
pub use strum::{EnumCount, IntoEnumIterator};
pub use version::NoteVersion;

/// Format for serializing SRS and proving/verifying keys.
pub const SERDE_FORMAT: SerdeFormat = SerdeFormat::Processed;

pub type AssignedCell = halo2_proofs::circuit::AssignedCell<Fr, Fr>;
pub type Value = halo2_proofs::circuit::Value<Fr>;

pub trait ProverKnowledge: Clone + PublicInputProvider<Self::PublicInput> {
    /// Associated type for the circuit.
    type Circuit: Circuit<Fr> + Clone + Debug + Default;

    /// Associated type for the public inputs. Expected to be iterable enumeration.
    type PublicInput: IntoEnumIterator + EnumCount;

    /// Creates a new instance of the circuit values with correct, randomized values. The circuit
    /// MUST be satisfied. Implementation might require more effort to generate such values. Useful
    /// for testing validity of the circuit constraints.
    fn random_correct_example(rng: &mut impl RngCore) -> Self;

    /// Creates a new instance of the circuit based on the prover's knowledge.
    fn create_circuit(&self) -> Self::Circuit;
}

pub trait PublicInputProvider<Id: IntoEnumIterator + EnumCount> {
    /// Computes specific public input value.
    fn compute_public_input(&self, input: Id) -> Fr;

    /// Return full public input as a vector of field elements.
    fn serialize_public_input(&self) -> Vec<Fr> {
        Id::iter()
            .map(|instance_id| self.compute_public_input(instance_id))
            .collect()
    }
}

impl<Id: IntoEnumIterator + EnumCount, Comp: Fn(Id) -> Fr> PublicInputProvider<Id> for Comp {
    fn compute_public_input(&self, instance_id: Id) -> Fr {
        self(instance_id)
    }
}
