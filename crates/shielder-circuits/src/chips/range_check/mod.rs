use gate::RangeCheckGate;
use halo2_proofs::plonk::{Advice, ConstraintSystem, Error};

use crate::{
    chips::{
        range_check::{gate::RangeCheckGateInput, running_sum::running_sum},
        sum::SumChip,
    },
    column_pool::{ColumnPool, ConfigPhase},
    consts::RANGE_PROOF_CHUNK_SIZE,
    embed::Embed,
    gates::Gate,
    synthesizer::Synthesizer,
    AssignedCell, Fr,
};

mod bits;
mod gate;
mod running_sum;

#[derive(Clone, Debug)]
pub struct RangeCheckChip {
    range_gate: RangeCheckGate,
    sum_chip: SumChip,
}

impl RangeCheckChip {
    pub fn new(
        system: &mut ConstraintSystem<Fr>,
        advice_pool: &mut ColumnPool<Advice, ConfigPhase>,
        sum_chip: SumChip,
    ) -> Self {
        Self {
            range_gate: RangeCheckGate::create_gate(system, advice_pool),
            sum_chip,
        }
    }

    /// Constrains the value to be less than `2^(CHUNK_SIZE * CHUNKS)`.
    pub fn constrain_value<const CHUNKS: usize>(
        &self,
        synthesizer: &mut impl Synthesizer,
        value: AssignedCell,
    ) -> Result<(), Error> {
        // PROVER STEPS:
        // 1. Represent `value` as a running sum (compute it outside of the circuit).
        let running_sum_off_circuit =
            running_sum(value.value().copied(), RANGE_PROOF_CHUNK_SIZE, CHUNKS);
        // 2. Embed the running sum into the circuit.
        let running_sum_cells = running_sum_off_circuit.embed(synthesizer, "running_sum")?;

        // VERIFIER CHECKS:
        // 1. Ensure that the running sum has proper length (off-circuit sanity check).
        assert_eq!(running_sum_off_circuit.len(), CHUNKS + 1);
        // 2. Ensure that the first sum is exactly `value`.
        self.sum_chip
            .constrain_equal(synthesizer, value, running_sum_cells[0].clone())?;
        // 3. Ensure that the last sum is zero.
        self.sum_chip
            .constrain_zero(synthesizer, running_sum_cells[CHUNKS].clone())?;
        // 4. Ensure that the running sum is correctly computed.
        for i in 0..CHUNKS {
            self.range_gate.apply_in_new_region(
                synthesizer,
                RangeCheckGateInput {
                    base: running_sum_cells[i].clone(),
                    shifted: running_sum_cells[i + 1].clone(),
                },
            )?;
        }

        Ok(())
    }
}
