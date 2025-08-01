use alloc::vec;

use halo2_proofs::{
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use macros::embeddable;

use crate::{
    column_pool::{AccessColumn, ColumnPool, ConfigPhase},
    embed::Embed,
    gates::{ensure_unique_columns, Gate},
    synthesizer::Synthesizer,
    AssignedCell, Fr,
};

/// Represents the relation: `(needle - haystack_1) · … · (needle - haystack_N) = 0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MembershipGate<const N: usize> {
    needle_advice: Column<Advice>,
    haystack_advice: [Column<Advice>; N],
    selector: Selector,
}

#[derive(Clone, Debug)]
#[embeddable(
    receiver = "MembershipGateInput<Fr, N>",
    impl_generics = "<const N: usize>",
    embedded = "MembershipGateInput<AssignedCell, N>"
)]
pub struct MembershipGateInput<T, const N: usize> {
    pub needle: T,
    pub haystack: [T; N],
}

const SELECTOR_OFFSET: usize = 0;
const ADVICE_OFFSET: usize = 0;
const GATE_NAME: &str = "Membership gate";

impl<const N: usize> Gate for MembershipGate<N> {
    type Input = MembershipGateInput<AssignedCell, N>;
    type Advice = (Column<Advice>, [Column<Advice>; N]);

    /// The gate operates on a single advice column `needle` and `N` advice columns `haystack`. It
    /// enforces that:
    ///
    /// `(needle[x] - haystack_1[x]) · … · (needle[x] - haystack_N[x]) = 0`, where `x` is the row
    /// where the gate is enabled.
    fn create_gate_custom(
        cs: &mut ConstraintSystem<Fr>,
        (needle_advice, haystack_advice): Self::Advice,
    ) -> Self {
        ensure_unique_columns(&[haystack_advice.to_vec(), vec![needle_advice]].concat());
        let selector = cs.selector();

        cs.create_gate(GATE_NAME, |vc| {
            let needle = vc.query_advice(needle_advice, Rotation(ADVICE_OFFSET as i32));
            let selector = vc.query_selector(selector);

            [haystack_advice.iter().fold(selector, |product, &hay| {
                let element = vc.query_advice(hay, Rotation(ADVICE_OFFSET as i32));
                product * (needle.clone() - element)
            })]
        });

        Self {
            needle_advice,
            haystack_advice,
            selector,
        }
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

                input.needle.copy_advice(
                    || "needle",
                    &mut region,
                    self.needle_advice,
                    ADVICE_OFFSET,
                )?;

                for (i, hay) in input.haystack.iter().enumerate() {
                    hay.copy_advice(
                        || alloc::format!("haystack_{i}"),
                        &mut region,
                        self.haystack_advice[i],
                        ADVICE_OFFSET,
                    )?;
                }

                Ok(())
            },
        )
    }

    fn organize_advice_columns(
        pool: &mut ColumnPool<Advice, ConfigPhase>,
        cs: &mut ConstraintSystem<Fr>,
    ) -> Self::Advice {
        pool.ensure_capacity(cs, N + 1);
        let haystack_advice = pool.get_column_array();
        let needle_advice = pool.get_column(N);
        (needle_advice, haystack_advice)
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{halo2curves::bn256::Fr, plonk::ConstraintSystem};

    use super::{MembershipGate, MembershipGateInput};
    use crate::gates::{test_utils::verify, Gate};

    #[test]
    fn gate_creation_with_proper_columns_passes() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let advice = (cs.advice_column(), [cs.advice_column(), cs.advice_column()]);
        MembershipGate::<2>::create_gate_custom(&mut cs, advice);
    }

    #[test]
    #[should_panic = "Advice columns must be unique"]
    fn needle_column_belongs_to_haystack_fails() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let col_1 = cs.advice_column();
        let col_2 = cs.advice_column();
        let improper_advice = (col_1, [col_1, col_2]);
        MembershipGate::<2>::create_gate_custom(&mut cs, improper_advice);
    }

    #[test]
    #[should_panic = "Advice columns must be unique"]
    fn haystack_does_not_have_distinct_columns_fails() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let col_1 = cs.advice_column();
        let col_2 = cs.advice_column();
        let improper_advice = (col_1, [col_2, col_2]);
        MembershipGate::<2>::create_gate_custom(&mut cs, improper_advice);
    }

    fn input(needle: impl Into<Fr>, [h0, h1]: [impl Into<Fr>; 2]) -> MembershipGateInput<Fr, 2> {
        MembershipGateInput {
            needle: needle.into(),
            haystack: [h0.into(), h1.into()],
        }
    }

    impl Default for MembershipGateInput<Fr, 2> {
        fn default() -> Self {
            Self {
                needle: Fr::default(),
                haystack: [Fr::default(), Fr::default()],
            }
        }
    }

    #[test]
    fn simple_case_passes() {
        assert!(verify::<MembershipGate<2>, _>(input(1, [2, 1])).is_ok());
    }

    #[test]
    fn needle_is_not_in_haystack_fails() {
        let err = verify::<MembershipGate<2>, _>(input(1, [2, 3])).expect_err("Should fail");
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("Constraint 0 in gate 0 ('Membership gate') is not satisfied"));
    }
}
