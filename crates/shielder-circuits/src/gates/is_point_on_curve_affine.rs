use alloc::vec;

use halo2_proofs::{
    arithmetic::CurveExt,
    halo2curves::{bn256::Fr, grumpkin::G1},
    plonk::{Advice, Column, ConstraintSystem, Constraints, Error, Expression, Selector},
    poly::Rotation,
};

use super::copy_affine_grumpkin_advices;
use crate::{
    column_pool::{AccessColumn, ColumnPool, ConfigPhase},
    curve_arithmetic::GrumpkinPointAffine,
    gates::{ensure_unique_columns, Gate},
    synthesizer::Synthesizer,
    AssignedCell,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IsPointOnCurveAffineGate {
    pub selector: Selector,
    pub point: [Column<Advice>; 2],
}

const SELECTOR_OFFSET: i32 = 0;
const ADVICE_OFFSET: i32 = 0;
const GATE_NAME: &str = "IsPointOnCurveAffine";

impl Gate for IsPointOnCurveAffineGate {
    type Input = GrumpkinPointAffine<AssignedCell>;

    type Advice = [Column<Advice>; 2];

    /// The gate checks whether a set of point coordinates satisfies the projective closure of the Grumpkin curve:
    /// y^2 = x^3 - 17
    fn create_gate_custom(cs: &mut ConstraintSystem<Fr>, point: Self::Advice) -> Self {
        ensure_unique_columns(point.as_ref());
        let selector = cs.selector();

        cs.create_gate(GATE_NAME, |vc| {
            let b = Expression::Constant(G1::b());
            let x = vc.query_advice(point[0], Rotation(ADVICE_OFFSET));
            let y = vc.query_advice(point[1], Rotation(ADVICE_OFFSET));

            Constraints::with_selector(
                vc.query_selector(selector),
                vec![(
                    "y^2 = x^3 - 17",
                    y.square() - x.clone() * x.clone() * x.clone() - b,
                )],
            )
        });

        Self { selector, point }
    }

    fn apply_in_new_region(
        &self,
        synthesizer: &mut impl Synthesizer,
        point: Self::Input,
    ) -> Result<(), Error> {
        synthesizer.assign_region(
            || GATE_NAME,
            |mut region| {
                self.selector
                    .enable(&mut region, SELECTOR_OFFSET as usize)?;

                copy_affine_grumpkin_advices(
                    &point,
                    "point",
                    &mut region,
                    self.point,
                    ADVICE_OFFSET as usize,
                )?;

                Ok(())
            },
        )
    }

    fn organize_advice_columns(
        pool: &mut ColumnPool<Advice, ConfigPhase>,
        cs: &mut ConstraintSystem<Fr>,
    ) -> Self::Advice {
        pool.ensure_capacity(cs, 2);
        [pool.get_column(0), pool.get_column(1)]
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use halo2_proofs::{
        dev::{MockProver, VerifyFailure},
        halo2curves::{bn256::Fr, ff::PrimeField},
    };

    use super::IsPointOnCurveAffineGate;
    use crate::{curve_arithmetic::GrumpkinPointAffine, gates::test_utils::OneGateCircuit, rng};

    fn verify(input: GrumpkinPointAffine<Fr>) -> Result<(), Vec<VerifyFailure>> {
        let circuit = OneGateCircuit::<IsPointOnCurveAffineGate, _>::new(input);
        MockProver::run(4, &circuit, vec![])
            .expect("Mock prover should run")
            .verify()
    }

    #[test]
    fn is_random_point_on_curve() {
        let mut rng = rng();
        let point: GrumpkinPointAffine<Fr> = GrumpkinPointAffine::random(&mut rng);
        assert!(verify(point).is_ok());
    }

    #[test]
    fn incorrect_inputs() {
        let point: GrumpkinPointAffine<Fr> =
            GrumpkinPointAffine::new(Fr::from_u128(1), Fr::from_u128(2));
        assert!(verify(point).is_err());
    }
}
