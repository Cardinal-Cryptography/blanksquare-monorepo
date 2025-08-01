use alloc::vec;

use halo2_proofs::{
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use macros::embeddable;

use crate::{
    column_pool::{AccessColumn, ConfigPhase},
    embed::Embed,
    gates::{ensure_unique_columns, Gate},
    synthesizer::Synthesizer,
    AssignedCell, Fr,
};

/// Represents the relation: `a + b = c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SumGate {
    advice: [Column<Advice>; 3],
    selector: Selector,
}

#[derive(Clone, Debug, Default)]
#[embeddable(receiver = "SumGateInput<Fr>", embedded = "SumGateInput<AssignedCell>")]
pub struct SumGateInput<T> {
    pub summand_1: T,
    pub summand_2: T,
    pub sum: T,
}

const SELECTOR_OFFSET: usize = 0;
const ADVICE_OFFSET: usize = 0;
const GATE_NAME: &str = "Sum gate";

impl Gate for SumGate {
    type Input = SumGateInput<AssignedCell>;
    type Advice = [Column<Advice>; 3];

    /// The gate operates on three advice columns `A`, `B`, and `C`. It enforces that:
    /// `A[x] + B[x] = C[x]`, where `x` is the row where the gate is enabled.
    fn create_gate_custom(cs: &mut ConstraintSystem<Fr>, advice: Self::Advice) -> Self {
        ensure_unique_columns(&advice);
        let selector = cs.selector();

        cs.create_gate(GATE_NAME, |vc| {
            let selector = vc.query_selector(selector);
            let summand_1 = vc.query_advice(advice[0], Rotation(ADVICE_OFFSET as i32));
            let summand_2 = vc.query_advice(advice[1], Rotation(ADVICE_OFFSET as i32));
            let sum = vc.query_advice(advice[2], Rotation(ADVICE_OFFSET as i32));
            vec![selector * (summand_1 + summand_2 - sum)]
        });
        Self { advice, selector }
    }

    fn apply_in_new_region(
        &self,
        synthesizer: &mut impl Synthesizer,
        input: Self::Input,
    ) -> Result<(), Error> {
        synthesizer.assign_region(
            || GATE_NAME,
            |mut region| {
                self.selector.enable(&mut region, SELECTOR_OFFSET)?;

                for (idx, (cell, name, offset)) in [
                    (&input.summand_1, "summand 1", ADVICE_OFFSET),
                    (&input.summand_2, "summand 2", ADVICE_OFFSET),
                    (&input.sum, "sum", ADVICE_OFFSET),
                ]
                .into_iter()
                .enumerate()
                {
                    cell.copy_advice(|| name, &mut region, self.advice[idx], offset)?;
                }

                Ok(())
            },
        )
    }

    fn organize_advice_columns(
        pool: &mut crate::column_pool::ColumnPool<Advice, ConfigPhase>,
        cs: &mut ConstraintSystem<Fr>,
    ) -> Self::Advice {
        pool.ensure_capacity(cs, 3);
        pool.get_column_array()
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{halo2curves::bn256::Fr, plonk::ConstraintSystem};

    use crate::gates::{
        sum::{SumGate, SumGateInput},
        test_utils::verify,
        Gate as _,
    };

    fn input(
        summand_1: impl Into<Fr>,
        summand_2: impl Into<Fr>,
        sum: impl Into<Fr>,
    ) -> SumGateInput<Fr> {
        SumGateInput {
            summand_1: summand_1.into(),
            summand_2: summand_2.into(),
            sum: sum.into(),
        }
    }

    #[test]
    fn gate_creation_with_proper_columns_passes() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let advice = [cs.advice_column(), cs.advice_column(), cs.advice_column()];
        SumGate::create_gate_custom(&mut cs, advice);
    }

    #[test]
    #[should_panic = "Advice columns must be unique"]
    fn gate_creation_with_not_distinct_columns_fails() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let advice_column = cs.advice_column();
        SumGate::create_gate_custom(&mut cs, [advice_column; 3]);
    }

    #[test]
    fn zeros_passes() {
        assert!(verify::<SumGate, _>(input(0, 0, 0)).is_ok());
    }

    #[test]
    fn simple_addition_passes() {
        assert!(verify::<SumGate, _>(input(1, 2, 3)).is_ok());
    }

    #[test]
    fn negation_passes() {
        assert!(verify::<SumGate, _>(input(5, Fr::from(5).neg(), 0)).is_ok());
    }

    #[test]
    fn incorrect_sum_fails() {
        let errors = verify::<SumGate, _>(input(2, 2, 3)).expect_err("Verification should fail");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Constraint 0 in gate 0 ('Sum gate') is not satisfied"));
    }
}
