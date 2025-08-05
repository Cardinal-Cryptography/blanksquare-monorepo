use alloc::vec;

use halo2_proofs::{
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};
use macros::embeddable;

use crate::{
    column_pool::{AccessColumn, ColumnPool, ConfigPhase},
    consts::RANGE_PROOF_CHUNK_SIZE,
    embed::Embed,
    gates::Gate,
    range_table::RangeTable,
    synthesizer::Synthesizer,
    AssignedCell, Fr,
};

/// Represents inequality: `base - shifted * 2^RANGE_PROOF_CHUNK_SIZE < 2^RANGE_PROOF_CHUNK_SIZE`.
#[derive(Clone, Debug)]
pub struct RangeCheckGate {
    advice: Column<Advice>,
    selector: Selector,
    table: RangeTable<{ RANGE_PROOF_CHUNK_SIZE }>,
}

/// The values that are required to construct a range check gate. Pair `(base, shifted)` is expected
/// to satisfy the inequality: `base - shifted * 2^CHUNK_SIZE < 2^CHUNK_SIZE`.
#[derive(Clone, Debug, Default)]
#[embeddable(
    receiver = "RangeCheckGateInput<Fr>",
    embedded = "RangeCheckGateInput<AssignedCell>"
)]
pub struct RangeCheckGateInput<T> {
    pub base: T,
    pub shifted: T,
}

const GATE_NAME: &str = "Range check gate";
const BASE_OFFSET: usize = 0;
const SHIFTED_OFFSET: usize = 1;

impl Gate for RangeCheckGate {
    type Input = RangeCheckGateInput<AssignedCell>;
    type Advice = Column<Advice>;

    /// The gate operates on a single advice column `A` and a table `T`. It enforces that:
    ///
    /// `A[x] - A[x+1] * 2^CHUNK_SIZE` belongs to `T`
    ///
    /// where:
    ///  - `x` is the row where the gate is enabled
    ///  - `T` represents set `[0, 2^CHUNK_SIZE)`
    fn create_gate_custom(cs: &mut ConstraintSystem<Fr>, advice: Self::Advice) -> Self {
        let selector = cs.complex_selector();
        let table = RangeTable::new(cs);

        cs.lookup("Range check lookup", |meta| {
            let selector = meta.query_selector(selector);
            let base = meta.query_advice(advice, Rotation(BASE_OFFSET as i32));
            let shifted = meta.query_advice(advice, Rotation(SHIFTED_OFFSET as i32));

            // We require that:
            //  - base = shifted * SCALE + chunk
            //  - chunk < 2^CHUNK_SIZE
            // where SCALE = 2^CHUNK_SIZE.
            //
            // Therefore, we recover the chunk as:
            //  - chunk = base - shifted * SCALE
            let scale = Expression::Constant(Fr::from(1 << RANGE_PROOF_CHUNK_SIZE));
            let chunk = base - shifted * scale;

            vec![(selector * chunk, table.column())]
        });

        RangeCheckGate {
            advice,
            selector,
            table,
        }
    }

    fn apply_in_new_region(
        &self,
        synthesizer: &mut impl Synthesizer,
        RangeCheckGateInput { base, shifted }: Self::Input,
    ) -> Result<(), Error> {
        self.table.ensure_initialized(synthesizer)?;
        synthesizer.assign_region(
            || GATE_NAME,
            |mut region| {
                self.selector.enable(&mut region, 0)?;
                base.copy_advice(|| "base", &mut region, self.advice, BASE_OFFSET)?;
                shifted.copy_advice(|| "shifted", &mut region, self.advice, SHIFTED_OFFSET)?;
                Ok(())
            },
        )?;

        Ok(())
    }

    fn organize_advice_columns(
        pool: &mut ColumnPool<Advice, ConfigPhase>,
        cs: &mut ConstraintSystem<Fr>,
    ) -> Self::Advice {
        pool.ensure_capacity(cs, 1);
        pool.get_any_column()
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::halo2curves::bn256::Fr;

    use crate::{
        chips::range_check::gate::{RangeCheckGate, RangeCheckGateInput},
        consts::RANGE_PROOF_CHUNK_SIZE,
        gates::test_utils::verify,
    };

    fn input(base: impl Into<Fr>, shifted: impl Into<Fr>) -> RangeCheckGateInput<Fr> {
        RangeCheckGateInput {
            base: base.into(),
            shifted: shifted.into(),
        }
    }

    #[test]
    fn zeros_pass() {
        assert!(verify::<RangeCheckGate, _>(input(0, 0)).is_ok());
    }

    #[test]
    fn full_range_passes() {
        let shifted = Fr::from(41);
        let scale = Fr::from(1 << RANGE_PROOF_CHUNK_SIZE);
        for i in 0..(1 << RANGE_PROOF_CHUNK_SIZE) {
            let base = shifted * scale + Fr::from(i);
            assert!(verify::<RangeCheckGate, _>(input(base, shifted)).is_ok());
        }
    }

    #[test]
    fn minimal_incorrect_difference_fails() {
        let shifted = Fr::from(42);
        let scale = Fr::from(1 << RANGE_PROOF_CHUNK_SIZE);
        let base = shifted * scale + scale;

        let err = verify::<RangeCheckGate, _>(input(base, shifted)).unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("Lookup Range check lookup"));
    }

    #[test]
    fn big_difference_fails() {
        let shifted = Fr::from(43);
        let base = Fr::from(40);

        let err = verify::<RangeCheckGate, _>(input(base, shifted)).unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("Lookup Range check lookup"));
    }

    #[test]
    fn one_below_limit_fails() {
        let shifted = Fr::from(44);
        let scale = Fr::from(1 << RANGE_PROOF_CHUNK_SIZE);
        let base = shifted * scale - Fr::from(1);

        let err = verify::<RangeCheckGate, _>(input(base, shifted)).unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("Lookup Range check lookup"));
    }
}
