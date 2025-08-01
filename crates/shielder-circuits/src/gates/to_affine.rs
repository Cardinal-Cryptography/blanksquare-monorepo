use alloc::vec;

use halo2_proofs::{
    arithmetic::Field,
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, ConstraintSystem, Constraints, Error, Expression, Selector},
    poly::Rotation,
};
use macros::embeddable;

use super::{copy_affine_grumpkin_advices, copy_grumpkin_advices};
use crate::{
    column_pool::{AccessColumn, ColumnPool, ConfigPhase},
    curve_arithmetic::{GrumpkinPoint, GrumpkinPointAffine},
    embed::Embed,
    gates::{ensure_unique_columns, Gate},
    synthesizer::Synthesizer,
    AssignedCell,
};

/// represents the relation:
/// P_projective (x, y, z) -> P_affine(x/z, y/z)
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToAffineGate {
    point_projective: [Column<Advice>; 3],
    point_affine: [Column<Advice>; 2],
    z_inverse: Column<Advice>,
    selector: Selector,
}

#[derive(Clone, Debug, Default)]
#[embeddable(
    receiver = "ToAffineGateInput<Fr>",
    embedded = "ToAffineGateInput<crate::AssignedCell>"
)]
pub struct ToAffineGateInput<T> {
    pub point_projective: GrumpkinPoint<T>,
    pub point_affine: GrumpkinPointAffine<T>,
    pub point_projective_z_inverse: T,
}

const SELECTOR_OFFSET: usize = 0;
const ADVICE_OFFSET: i32 = 0;
const GATE_NAME: &str = "To affine gate";

impl Gate for ToAffineGate {
    type Input = ToAffineGateInput<AssignedCell>;

    type Advice = (
        [Column<Advice>; 3], // projective
        [Column<Advice>; 2], // affine
        Column<Advice>,      // z_projective^-1
    );

    fn create_gate_custom(
        cs: &mut ConstraintSystem<Fr>,
        (point_projective, point_affine, z_inverse): Self::Advice,
    ) -> Self {
        ensure_unique_columns(
            &[
                point_affine.to_vec(),
                point_projective.to_vec(),
                vec![z_inverse],
            ]
            .concat(),
        );
        let selector = cs.selector();

        cs.create_gate(GATE_NAME, |vc| {
            let selector = vc.query_selector(selector);

            let x_projective = vc.query_advice(point_projective[0], Rotation(ADVICE_OFFSET));
            let y_projective = vc.query_advice(point_projective[1], Rotation(ADVICE_OFFSET));
            let z_projective = vc.query_advice(point_projective[2], Rotation(ADVICE_OFFSET));

            let x_affine = vc.query_advice(point_affine[0], Rotation(ADVICE_OFFSET));
            let y_affine = vc.query_advice(point_affine[1], Rotation(ADVICE_OFFSET));

            let z_inverse = vc.query_advice(z_inverse, Rotation(ADVICE_OFFSET));

            Constraints::with_selector(
                selector,
                vec![
                    x_affine - x_projective * z_inverse.clone(),
                    y_affine - y_projective * z_inverse.clone(),
                    z_projective * z_inverse - Expression::Constant(Fr::ONE),
                ],
            )
        });

        Self {
            point_projective,
            point_affine,
            z_inverse,
            selector,
        }
    }

    fn apply_in_new_region(
        &self,
        synthesizer: &mut impl Synthesizer,
        ToAffineGateInput {
            point_projective,
            point_affine,
            point_projective_z_inverse: z_inverse,
        }: Self::Input,
    ) -> Result<(), Error> {
        synthesizer.assign_region(
            || GATE_NAME,
            |mut region| {
                self.selector.enable(&mut region, SELECTOR_OFFSET)?;

                copy_grumpkin_advices(
                    &point_projective,
                    "point_projective",
                    &mut region,
                    self.point_projective,
                    ADVICE_OFFSET as usize,
                )?;

                copy_affine_grumpkin_advices(
                    &point_affine,
                    "point_affine",
                    &mut region,
                    self.point_affine,
                    ADVICE_OFFSET as usize,
                )?;

                z_inverse.copy_advice(
                    || "z_inverse",
                    &mut region,
                    self.z_inverse,
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
        pool.ensure_capacity(cs, 6);

        (
            [pool.get_column(0), pool.get_column(1), pool.get_column(2)], // projective
            [pool.get_column(3), pool.get_column(4)],                     // affine
            pool.get_column(5),                                           // z_{p}^{-1}
        )
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use halo2_proofs::{
        dev::{MockProver, VerifyFailure},
        halo2curves::{bn256::Fr, group::Group, grumpkin::G1},
    };

    use super::*;
    use crate::{curve_arithmetic, gates::test_utils::OneGateCircuit, rng};

    fn input(
        point_projective: GrumpkinPoint<Fr>,
        point_affine: GrumpkinPointAffine<Fr>,
        point_projective_z_inverse: Fr,
    ) -> ToAffineGateInput<Fr> {
        ToAffineGateInput {
            point_projective,
            point_affine,
            point_projective_z_inverse,
        }
    }

    fn verify(input: ToAffineGateInput<Fr>) -> Result<(), Vec<VerifyFailure>> {
        let circuit = OneGateCircuit::<ToAffineGate, _>::new(input);
        MockProver::run(3, &circuit, vec![])
            .expect("Mock prover should run")
            .verify()
    }

    #[test]
    fn gate_creation() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let p = [cs.advice_column(), cs.advice_column(), cs.advice_column()];
        let a = [cs.advice_column(), cs.advice_column()];
        let z_inverse = cs.advice_column();

        ToAffineGate::create_gate_custom(&mut cs, (p, a, z_inverse));
    }

    #[test]
    #[should_panic = "Advice columns must be unique"]
    fn unique_columns() {
        let mut cs = ConstraintSystem::<Fr>::default();

        let col = cs.advice_column();
        let p = [col, cs.advice_column(), cs.advice_column()];
        let a = [cs.advice_column(), cs.advice_column()];
        let z_inverse = col;

        ToAffineGate::create_gate_custom(&mut cs, (p, a, z_inverse));
    }

    #[test]
    fn coordinate_conversion() {
        let rng = rng();

        let point_projective: GrumpkinPoint<Fr> = G1::random(rng).into();
        let point_affine: GrumpkinPointAffine<Fr> = point_projective.into();
        let z_inverse = point_projective
            .z
            .invert()
            .expect("z coordinate has an inverse");

        assert!(verify(input(point_projective, point_affine, z_inverse)).is_ok());
    }

    #[test]
    fn incorrect_inputs() {
        let rng = rng();

        let point_projective: GrumpkinPoint<Fr> = G1::random(&mut rng.clone()).into();

        let point_affine: GrumpkinPointAffine<Fr> =
            curve_arithmetic::normalize_point(curve_arithmetic::point_double(point_projective))
                .into();

        let z_inverse = point_projective
            .z
            .invert()
            .expect("z coordinate has an inverse");

        assert!(verify(input(point_projective, point_affine, z_inverse)).is_err());
    }
}
