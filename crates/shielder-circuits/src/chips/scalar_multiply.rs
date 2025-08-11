use alloc::vec::Vec;

use halo2_proofs::plonk::Error;

use crate::{
    consts::FIELD_BITS,
    curve_arithmetic::{self, GrumpkinPoint},
    embed::Embed,
    gates::{
        scalar_multiply::{ScalarMultiplyGate, ScalarMultiplyGateInput},
        Gate,
    },
    synthesizer::Synthesizer,
    AssignedCell, V,
};

#[derive(Clone, Debug)]
pub struct ScalarMultiplyChipInput<T> {
    /// point on the Grumpkin curve
    pub input: GrumpkinPoint<T>,
    /// scalar bits in LE representation
    pub scalar_bits: [T; FIELD_BITS],
}

impl<T: Default + Copy> Default for ScalarMultiplyChipInput<T> {
    fn default() -> Self {
        Self {
            input: GrumpkinPoint::default(),
            scalar_bits: [T::default(); FIELD_BITS],
        }
    }
}

/// Chip that computes the result of adding a point P on the Grumpkin curve to itself n times.
///
/// n * P = S
#[derive(Clone, Debug)]
pub struct ScalarMultiplyChip {
    pub multiply_gate: ScalarMultiplyGate,
}

impl ScalarMultiplyChip {
    pub fn new(multiply_gate: ScalarMultiplyGate) -> Self {
        Self { multiply_gate }
    }

    pub fn scalar_multiply(
        &self,
        synthesizer: &mut impl Synthesizer,
        inputs: &ScalarMultiplyChipInput<AssignedCell>,
    ) -> Result<GrumpkinPoint<AssignedCell>, Error> {
        let ScalarMultiplyChipInput { scalar_bits, input } = inputs;

        let bits: Vec<V> = scalar_bits
            .iter()
            .map(|cell| V(cell.value().cloned()))
            .collect();
        let bits: [V; FIELD_BITS] = bits.try_into().expect("not a {FIELD_BITS} bit array");
        let input: GrumpkinPoint<V> = GrumpkinPoint {
            x: V(input.x.value().cloned()),
            y: V(input.y.value().cloned()),
            z: V(input.z.value().cloned()),
        };

        let final_result_value: GrumpkinPoint<V> = curve_arithmetic::scalar_multiply(input, bits);
        let final_result = final_result_value.embed(synthesizer, "S")?;

        self.multiply_gate.apply_in_new_region(
            synthesizer,
            ScalarMultiplyGateInput {
                scalar_bits: scalar_bits.clone(),
                input: inputs.input.clone(),
                final_result: final_result.clone(),
            },
        )?;

        Ok(final_result)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    use halo2_proofs::{
        circuit::{floor_planner::V1, Layouter},
        dev::MockProver,
        halo2curves::{bn256::Fr, ff::PrimeField, group::Group, grumpkin::G1},
        plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
    };

    use super::{ScalarMultiplyChip, ScalarMultiplyChipInput};
    use crate::{
        column_pool::{ColumnPool, PreSynthesisPhase},
        config_builder::ConfigsBuilder,
        consts::FIELD_BITS,
        curve_arithmetic::{self, field_element_to_le_bits},
        embed::Embed,
        rng,
        synthesizer::create_synthesizer,
        GrumpkinPoint,
    };

    #[derive(Clone, Debug, Default)]
    struct ScalarMultiplyCircuit(ScalarMultiplyChipInput<Fr>);

    impl Circuit<Fr> for ScalarMultiplyCircuit {
        type Config = (
            ColumnPool<Advice, PreSynthesisPhase>,
            ScalarMultiplyChip,
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

            let fixed = meta.fixed_column();
            meta.enable_constant(fixed);

            // register chip
            let configs_builder = ConfigsBuilder::new(meta).with_scalar_multiply_chip();
            let chip = configs_builder.scalar_multiply_chip();

            (configs_builder.finish(), chip, instance)
        }

        fn synthesize(
            &self,
            (column_pool, chip, instance): Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let ScalarMultiplyChipInput { input, scalar_bits } = self.0;

            let column_pool = column_pool.start_synthesis();
            let mut synthesizer = create_synthesizer(&mut layouter, &column_pool);

            let input = input.embed(&mut synthesizer, "input")?;
            let scalar_bits = scalar_bits.embed(&mut synthesizer, "scalar_bits")?;

            let result = chip.scalar_multiply(
                &mut synthesizer,
                &ScalarMultiplyChipInput { input, scalar_bits },
            )?;

            synthesizer.constrain_instance(result.x.cell(), instance, 0)?;
            synthesizer.constrain_instance(result.y.cell(), instance, 1)?;
            synthesizer.constrain_instance(result.z.cell(), instance, 2)?;

            Ok(())
        }
    }

    fn input(p: G1, scalar_bits: [Fr; FIELD_BITS]) -> ScalarMultiplyChipInput<Fr> {
        ScalarMultiplyChipInput {
            input: p.into(),
            scalar_bits: scalar_bits.into(),
        }
    }

    fn verify(
        input: ScalarMultiplyChipInput<Fr>,
        expected: GrumpkinPoint<Fr>,
    ) -> Result<(), Vec<String>> {
        MockProver::run(
            10,
            &ScalarMultiplyCircuit(input),
            vec![vec![expected.x, expected.y, expected.z]],
        )
        .expect("Mock prover should run successfully")
        .verify()
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|failure| failure.to_string())
                .collect()
        })
    }

    #[test]
    fn multiply_random_point() {
        let rng = rng();
        let p = G1::random(rng.clone());
        let n = Fr::from_u128(3);
        let bits = field_element_to_le_bits(n);

        let expected = curve_arithmetic::scalar_multiply(p.into(), bits.clone());

        let input = input(p, bits);

        assert!(verify(input, expected).is_ok());
    }
}
