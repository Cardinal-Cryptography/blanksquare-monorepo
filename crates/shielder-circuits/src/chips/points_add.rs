use halo2_proofs::plonk::Error;

use crate::{
    curve_arithmetic::{self, GrumpkinPoint},
    embed::Embed,
    gates::{
        points_add::{PointsAddGate, PointsAddGateInput},
        Gate,
    },
    synthesizer::Synthesizer,
    AssignedCell, Value,
};

/// Chip that adds two points on a Grumpkin curve.
///
/// P + Q = S
#[derive(Clone, Debug)]
pub struct PointsAddChip {
    pub gate: PointsAddGate,
}

impl PointsAddChip {
    pub fn new(gate: PointsAddGate) -> Self {
        Self { gate }
    }

    pub fn points_add(
        &self,
        synthesizer: &mut impl Synthesizer,
        p: &GrumpkinPoint<AssignedCell>,
        q: &GrumpkinPoint<AssignedCell>,
    ) -> Result<GrumpkinPoint<AssignedCell>, Error> {
        let s_value = curve_arithmetic::points_add::<Value>(p.clone().into(), q.clone().into());
        let s = s_value.embed(synthesizer, "S")?;

        self.gate.apply_in_new_region(
            synthesizer,
            PointsAddGateInput {
                p: p.clone(),
                q: q.clone(),
                s: s.clone(),
            },
        )?;

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use std::{vec, vec::Vec};

    use halo2_proofs::{
        arithmetic::Field,
        circuit::{floor_planner::V1, Layouter},
        dev::{MockProver, VerifyFailure},
        halo2curves::{bn256::Fr, group::Group, grumpkin::G1},
        plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
    };

    use super::PointsAddChip;
    use crate::{
        column_pool::{ColumnPool, PreSynthesisPhase},
        config_builder::ConfigsBuilder,
        embed::Embed,
        rng,
        synthesizer::create_synthesizer,
        GrumpkinPoint,
    };

    #[derive(Clone, Debug, Default)]
    struct PointsAddCircuit {
        p: GrumpkinPoint<Fr>,
        q: GrumpkinPoint<Fr>,
    }

    impl Circuit<Fr> for PointsAddCircuit {
        type Config = (
            ColumnPool<Advice, PreSynthesisPhase>,
            PointsAddChip,
            Column<Instance>,
        );

        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            // public input column
            let instance = meta.instance_column();
            meta.enable_equality(instance);
            // register point add chip
            let configs_builder = ConfigsBuilder::new(meta).with_points_add_chip();
            let chip = configs_builder.points_add_chip();

            (configs_builder.finish(), chip, instance)
        }

        fn synthesize(
            &self,
            (column_pool, chip, instance): Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let column_pool = column_pool.start_synthesis();
            let mut synthesizer = create_synthesizer(&mut layouter, &column_pool);

            let p = self.p.embed(&mut synthesizer, "P")?;
            let q = self.q.embed(&mut synthesizer, "Q")?;

            let s = chip.points_add(&mut synthesizer, &p, &q)?;

            synthesizer.constrain_instance(s.x.cell(), instance, 0)?;
            synthesizer.constrain_instance(s.y.cell(), instance, 1)?;
            synthesizer.constrain_instance(s.z.cell(), instance, 2)?;

            Ok(())
        }
    }

    fn verify(
        p: GrumpkinPoint<Fr>,
        q: GrumpkinPoint<Fr>,
        expected: GrumpkinPoint<Fr>,
    ) -> Result<(), Vec<VerifyFailure>> {
        let circuit = PointsAddCircuit { p, q };
        MockProver::run(4, &circuit, vec![vec![expected.x, expected.y, expected.z]])
            .expect("Mock prover should run")
            .verify()
    }

    #[test]
    fn adding_points_at_infinity() {
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
        let expected = p + q;

        assert!(verify(p.into(), q.into(), expected.into()).is_ok());
    }

    #[test]
    fn adding_random_points() {
        let rng = rng();

        let p = G1::random(rng.clone());
        let q = G1::random(rng.clone());
        let expected = p + q;

        assert!(verify(p.into(), q.into(), expected.into()).is_ok());
    }

    #[test]
    fn incorrect_inputs() {
        let rng = rng();

        let p = G1::random(rng.clone());
        let q = G1::random(rng.clone());
        let s = G1::random(rng.clone());

        assert!(verify(p.into(), q.into(), s.into()).is_err());
    }
}
