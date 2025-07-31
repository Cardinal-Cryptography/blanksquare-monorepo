use halo2_proofs::plonk::Error;

use crate::{
    gates::{
        sum::{SumGate, SumGateInput},
        Gate,
    },
    synthesizer::Synthesizer,
    AssignedCell, Field, Fr,
};

#[derive(Clone, Debug)]
pub struct SumChip(SumGate);

impl SumChip {
    pub fn new(gate: SumGate) -> Self {
        Self(gate)
    }

    /// Constrain cells to satisfy the equation `summand_1 + summand_2 = sum`.
    pub fn constrain_sum(
        &self,
        synthesizer: &mut impl Synthesizer,
        summand_1: AssignedCell,
        summand_2: AssignedCell,
        sum: AssignedCell,
    ) -> Result<(), Error> {
        let gate_input = SumGateInput {
            summand_1,
            summand_2,
            sum,
        };
        self.0.apply_in_new_region(synthesizer, gate_input)
    }

    /// Constrain cells to satisfy the equation `left_sock = right_sock`.
    pub fn constrain_equal(
        &self,
        synthesizer: &mut impl Synthesizer,
        left_sock: AssignedCell,
        right_sock: AssignedCell,
    ) -> Result<(), Error> {
        let gate_input = SumGateInput {
            summand_1: left_sock,
            summand_2: self.zero(synthesizer)?,
            sum: right_sock,
        };
        self.0.apply_in_new_region(synthesizer, gate_input)
    }

    /// Constrain cell to satisfy the equation `zero = 0`.
    pub fn constrain_zero(
        &self,
        synthesizer: &mut impl Synthesizer,
        zero: AssignedCell,
    ) -> Result<(), Error> {
        let true_zero = self.zero(synthesizer)?;
        self.constrain_equal(synthesizer, zero, true_zero)
    }

    fn zero(&self, synthesizer: &mut impl Synthesizer) -> Result<AssignedCell, Error> {
        synthesizer.assign_constant("zero", Fr::ZERO)
    }
}
