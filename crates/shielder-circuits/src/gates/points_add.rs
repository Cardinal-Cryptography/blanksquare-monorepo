use alloc::vec;

use halo2_proofs::{
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, ConstraintSystem, Constraints, Error, Selector},
    poly::Rotation,
};
use macros::embeddable;

use super::copy_grumpkin_advices;
use crate::{
    column_pool::{AccessColumn, ColumnPool, ConfigPhase},
    curve_arithmetic::{self, GrumpkinPoint},
    embed::Embed,
    gates::{ensure_unique_columns, Gate},
    synthesizer::Synthesizer,
    AssignedCell,
};

/// represents the relation P + Q = S
///
/// where P,Q,S are points on the G1 of the Grumpkin curve
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PointsAddGate {
    pub p: [Column<Advice>; 3],
    pub q: [Column<Advice>; 3],
    pub s: [Column<Advice>; 3],
    pub selector: Selector,
}

#[derive(Clone, Debug, Default)]
#[embeddable(
    receiver = "PointsAddGateInput<Fr>",
    embedded = "PointsAddGateInput<crate::AssignedCell>"
)]
pub struct PointsAddGateInput<T> {
    pub p: GrumpkinPoint<T>, // x1,y1,z1
    pub q: GrumpkinPoint<T>, // x2,y2,z2
    pub s: GrumpkinPoint<T>, // x3,y3,z3
}

const SELECTOR_OFFSET: usize = 0;
const ADVICE_OFFSET: i32 = 0;
const GATE_NAME: &str = "Point add gate";

impl Gate for PointsAddGate {
    type Input = PointsAddGateInput<AssignedCell>;

    type Advice = (
        [Column<Advice>; 3], // p
        [Column<Advice>; 3], // q
        [Column<Advice>; 3], // s
    );

    fn create_gate_custom(cs: &mut ConstraintSystem<Fr>, (p, q, s): Self::Advice) -> Self {
        ensure_unique_columns(&[p.to_vec(), q.to_vec(), s.to_vec()].concat());
        let selector = cs.selector();

        cs.create_gate(GATE_NAME, |vc| {
            let selector = vc.query_selector(selector);

            let x1 = vc.query_advice(p[0], Rotation(ADVICE_OFFSET));
            let y1 = vc.query_advice(p[1], Rotation(ADVICE_OFFSET));
            let z1 = vc.query_advice(p[2], Rotation(ADVICE_OFFSET));

            let x2 = vc.query_advice(q[0], Rotation(ADVICE_OFFSET));
            let y2 = vc.query_advice(q[1], Rotation(ADVICE_OFFSET));
            let z2 = vc.query_advice(q[2], Rotation(ADVICE_OFFSET));

            let x3 = vc.query_advice(s[0], Rotation(ADVICE_OFFSET));
            let y3 = vc.query_advice(s[1], Rotation(ADVICE_OFFSET));
            let z3 = vc.query_advice(s[2], Rotation(ADVICE_OFFSET));

            let GrumpkinPoint {
                x: res_x3,
                y: res_y3,
                z: res_z3,
            } = curve_arithmetic::points_add(
                GrumpkinPoint::new(x1, y1, z1),
                GrumpkinPoint::new(x2, y2, z2),
            );

            Constraints::with_selector(selector, vec![res_x3 - x3, res_y3 - y3, res_z3 - z3])
        });

        Self { p, q, s, selector }
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

                copy_grumpkin_advices(&input.p, "P", &mut region, self.p, ADVICE_OFFSET as usize)?;
                copy_grumpkin_advices(&input.q, "Q", &mut region, self.q, ADVICE_OFFSET as usize)?;
                copy_grumpkin_advices(&input.s, "S", &mut region, self.s, ADVICE_OFFSET as usize)?;

                Ok(())
            },
        )
    }

    fn organize_advice_columns(
        pool: &mut ColumnPool<Advice, ConfigPhase>,
        cs: &mut ConstraintSystem<Fr>,
    ) -> Self::Advice {
        pool.ensure_capacity(cs, 9);

        (
            [pool.get_column(0), pool.get_column(1), pool.get_column(2)],
            [pool.get_column(3), pool.get_column(4), pool.get_column(5)],
            [pool.get_column(6), pool.get_column(7), pool.get_column(8)],
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{vec, vec::Vec};

    use halo2_proofs::{
        arithmetic::Field,
        dev::{MockProver, VerifyFailure},
        halo2curves::{bn256::Fr, group::Group, grumpkin::G1},
        plonk::ConstraintSystem,
    };

    use super::{PointsAddGate, PointsAddGateInput};
    use crate::{
        gates::{test_utils::OneGateCircuit, Gate as _},
        rng,
    };

    fn input(p: G1, q: G1, s: G1) -> PointsAddGateInput<Fr> {
        PointsAddGateInput {
            p: p.into(),
            q: q.into(),
            s: s.into(),
        }
    }

    fn verify(input: PointsAddGateInput<Fr>) -> Result<(), Vec<VerifyFailure>> {
        let circuit = OneGateCircuit::<PointsAddGate, _>::new(input);
        MockProver::run(3, &circuit, vec![])
            .expect("Mock prover should run")
            .verify()
    }

    #[test]
    fn gate_creation() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let p = [cs.advice_column(), cs.advice_column(), cs.advice_column()];
        let q = [cs.advice_column(), cs.advice_column(), cs.advice_column()];
        let s = [cs.advice_column(), cs.advice_column(), cs.advice_column()];

        PointsAddGate::create_gate_custom(&mut cs, (p, q, s));
    }

    #[test]
    #[should_panic = "Advice columns must be unique"]
    fn unique_columns() {
        let mut cs = ConstraintSystem::<Fr>::default();
        let col = cs.advice_column();
        let p = [col, cs.advice_column(), cs.advice_column()];
        let q = [cs.advice_column(), cs.advice_column(), cs.advice_column()];
        let s = [cs.advice_column(), col, cs.advice_column()];

        PointsAddGate::create_gate_custom(&mut cs, (p, q, s));
    }

    #[test]
    fn adding_point_at_infinity() {
        let p = G1 {
            x: Fr::ZERO,
            y: Fr::ONE,
            z: Fr::ZERO,
        };
        let q = G1 {
            x: Fr::ZERO,
            y: Fr::ONE,
            z: Fr::ZERO,
        };
        let s = p + q;

        assert!(verify(input(p, q, s)).is_ok());
    }

    #[test]
    fn adding_random_points() {
        let rng = rng();

        let p = G1::random(rng.clone());
        let q = G1::random(rng.clone());
        let s = p + q;

        assert!(verify(input(p, q, s)).is_ok());
    }

    #[test]
    fn incorrect_inputs() {
        let rng = rng();

        let p = G1::random(rng.clone());
        let q = G1::random(rng.clone());
        let s = G1::random(rng.clone());

        verify(input(p, q, s)).expect_err("Verification should fail");
    }
}
