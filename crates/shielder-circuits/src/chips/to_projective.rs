use halo2_proofs::{arithmetic::Field, halo2curves::bn256::Fr, plonk::Error};

use crate::{
    curve_arithmetic::{GrumpkinPoint, GrumpkinPointAffine},
    synthesizer::Synthesizer,
    AssignedCell,
};

/// Chip that converts between a point in affine to a point in projective coordinates
#[derive(Clone, Debug, Default)]
pub struct ToProjectiveChip;

impl ToProjectiveChip {
    pub fn new() -> Self {
        Self
    }

    pub fn to_projective(
        &self,
        synthesizer: &mut impl Synthesizer,
        point_affine: &GrumpkinPointAffine<AssignedCell>,
    ) -> Result<GrumpkinPoint<AssignedCell>, Error> {
        let GrumpkinPointAffine { x, y } = point_affine;
        let one = synthesizer.assign_constant("ONE", Fr::ONE)?;

        Ok(GrumpkinPoint {
            x: x.clone(),
            y: y.clone(),
            z: one,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use halo2_proofs::{
        circuit::{floor_planner::V1, Layouter},
        dev::{MockProver, VerifyFailure},
        halo2curves::bn256::Fr,
        plonk::{Advice, Circuit, Column, ConstraintSystem, Instance},
    };

    use super::*;
    use crate::{
        column_pool::{ColumnPool, PreSynthesisPhase},
        config_builder::ConfigsBuilder,
        curve_arithmetic,
        embed::Embed,
        rng,
        synthesizer::create_synthesizer,
    };

    #[derive(Clone, Debug, Default)]
    struct ToProjectiveCircuit(GrumpkinPointAffine<Fr>);

    impl Circuit<Fr> for ToProjectiveCircuit {
        type Config = (
            ColumnPool<Advice, PreSynthesisPhase>,
            ToProjectiveChip,
            Column<Instance>,
        );

        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            let instance = meta.instance_column();
            meta.enable_equality(instance);

            let fixed = meta.fixed_column();
            meta.enable_constant(fixed);

            let mut configs_builder = ConfigsBuilder::new(meta).with_to_projective_chip();
            configs_builder.advice_pool_with_capacity(5);

            let chip = configs_builder.to_projective_chip();
            (configs_builder.finish(), chip, instance)
        }

        fn synthesize(
            &self,
            (column_pool, chip, instance): Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let column_pool = column_pool.start_synthesis();

            let mut synthesizer = create_synthesizer(&mut layouter, &column_pool);

            let point_affine = self.0.embed(&mut synthesizer, "point_affine")?;
            let point_projective = chip.to_projective(&mut synthesizer, &point_affine)?;

            synthesizer.constrain_instance(point_projective.x.cell(), instance, 0)?;
            synthesizer.constrain_instance(point_projective.y.cell(), instance, 1)?;
            synthesizer.constrain_instance(point_projective.z.cell(), instance, 2)?;

            Ok(())
        }
    }

    fn verify(
        input: GrumpkinPointAffine<Fr>,
        expected: GrumpkinPoint<Fr>,
    ) -> Result<(), Vec<VerifyFailure>> {
        let circuit = ToProjectiveCircuit(input);
        MockProver::run(4, &circuit, vec![vec![expected.x, expected.y, expected.z]])
            .expect("Mock prover should run")
            .verify()
    }

    #[test]
    fn coordinate_conversion() {
        let mut rng = rng();

        let point_affine: GrumpkinPointAffine<Fr> = GrumpkinPointAffine::random(&mut rng).into();
        let point_projective = point_affine.clone().into();

        assert!(verify(point_affine, point_projective).is_ok());
    }

    #[test]
    fn incorrect_inputs() {
        let mut rng = rng();

        let point_affine: GrumpkinPointAffine<Fr> = GrumpkinPointAffine::random(&mut rng).into();
        let point_projective = curve_arithmetic::normalize_point(curve_arithmetic::point_double(
            point_affine.clone().into(),
        ));

        assert!(verify(point_affine, point_projective).is_err());
    }
}
