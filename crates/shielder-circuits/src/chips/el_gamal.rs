use halo2_proofs::{halo2curves::bn256::Fr, plonk::Error};
use macros::embeddable;

use super::{points_add::PointsAddChip, scalar_multiply::ScalarMultiplyChip, sum::SumChip};
use crate::{
    chips::scalar_multiply::ScalarMultiplyChipInput, consts::FIELD_BITS,
    curve_arithmetic::GrumpkinPoint, embed::Embed, synthesizer::Synthesizer, AssignedCell,
};

#[derive(Clone, Debug)]
#[embeddable(
    receiver = "ElGamalEncryptionInput<Fr>",
    embedded = "ElGamalEncryptionInput<AssignedCell>"
)]
pub struct ElGamalEncryptionInput<T> {
    pub message: GrumpkinPoint<T>,
    pub public_key: GrumpkinPoint<T>,
    pub salt_le_bits: [T; FIELD_BITS],
}

impl<T: Default + Copy> Default for ElGamalEncryptionInput<T> {
    fn default() -> Self {
        Self {
            message: GrumpkinPoint::default(),
            public_key: GrumpkinPoint::default(),
            salt_le_bits: [T::default(); FIELD_BITS],
        }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub struct ElGamalEncryptionChipOutput<T> {
    pub ciphertext1: GrumpkinPoint<T>,
    pub ciphertext2: GrumpkinPoint<T>,
}

#[derive(Clone, Debug)]
pub struct ElGamalEncryptionChip {
    pub multiply_chip: ScalarMultiplyChip,
    pub add_chip: PointsAddChip,
    pub sum_chip: SumChip,
}

impl ElGamalEncryptionChip {
    pub fn new(
        multiply_chip: ScalarMultiplyChip,
        add_chip: PointsAddChip,
        sum_chip: SumChip,
    ) -> Self {
        Self {
            multiply_chip,
            add_chip,
            sum_chip,
        }
    }

    fn constrain_generator(
        &self,
        synthesizer: &mut impl Synthesizer,
        generator: GrumpkinPoint<AssignedCell>,
    ) -> Result<(), Error> {
        let g = GrumpkinPoint::generator();

        let gx = synthesizer.assign_constant("g.x", g.x)?;
        let gy = synthesizer.assign_constant("g.y", g.y)?;
        let gz = synthesizer.assign_constant("g.z", g.z)?;

        self.sum_chip
            .constrain_equal(synthesizer, generator.x, gx)?;
        self.sum_chip
            .constrain_equal(synthesizer, generator.y, gy)?;
        self.sum_chip
            .constrain_equal(synthesizer, generator.z, gz)?;

        Ok(())
    }

    pub fn encrypt(
        &self,
        synthesizer: &mut impl Synthesizer,
        ElGamalEncryptionInput {
            message,
            public_key,
            salt_le_bits,
        }: &ElGamalEncryptionInput<AssignedCell>,
    ) -> Result<ElGamalEncryptionChipOutput<AssignedCell>, Error> {
        let generator_value = GrumpkinPoint::generator();
        let generator = generator_value.embed(synthesizer, "G1 generator")?;

        self.constrain_generator(synthesizer, generator.clone())?;

        let shared_secret = self.multiply_chip.scalar_multiply(
            synthesizer,
            &ScalarMultiplyChipInput {
                input: public_key.clone(),
                scalar_bits: salt_le_bits.clone(),
            },
        )?;

        let ciphertext1 = self.multiply_chip.scalar_multiply(
            synthesizer,
            &ScalarMultiplyChipInput {
                input: generator,
                scalar_bits: salt_le_bits.clone(),
            },
        )?;

        let ciphertext2 = self
            .add_chip
            .points_add(synthesizer, message, &shared_secret)?;

        Ok(ElGamalEncryptionChipOutput {
            ciphertext1,
            ciphertext2,
        })
    }
}

pub mod off_circuit {
    use halo2_proofs::{
        arithmetic::Field,
        halo2curves::{
            bn256::Fr,
            grumpkin::{self, G1},
        },
    };
    use rand::RngCore;

    use crate::{
        curve_arithmetic::{self, GrumpkinPoint},
        field_element_to_le_bits,
    };

    pub fn generate_keys(rng: &mut impl RngCore) -> (grumpkin::Fr, GrumpkinPoint<Fr>) {
        let generator = G1::generator();
        let private_key = grumpkin::Fr::random(rng);
        let private_key_bits = field_element_to_le_bits(private_key);

        let public_key = curve_arithmetic::normalize_point(curve_arithmetic::scalar_multiply(
            generator.into(),
            private_key_bits,
        ));

        (private_key, public_key)
    }

    pub fn encrypt(
        message: GrumpkinPoint<Fr>,
        public_key: GrumpkinPoint<Fr>,
        encryption_salt: grumpkin::Fr,
    ) -> (GrumpkinPoint<Fr>, GrumpkinPoint<Fr>) {
        let generator = GrumpkinPoint::generator();
        let salt_bits = field_element_to_le_bits(encryption_salt);
        let shared_secret = curve_arithmetic::scalar_multiply(public_key, salt_bits);
        let ciphertext1 = curve_arithmetic::scalar_multiply(generator, salt_bits);
        let ciphertext2 = curve_arithmetic::points_add(message, shared_secret);

        (ciphertext1, ciphertext2)
    }

    pub fn decrypt(
        ciphertext1: GrumpkinPoint<Fr>,
        ciphertext2: GrumpkinPoint<Fr>,
        private_key: grumpkin::Fr,
    ) -> GrumpkinPoint<Fr> {
        let private_key_bits = field_element_to_le_bits(private_key);
        let shared_secret = curve_arithmetic::scalar_multiply(ciphertext1, private_key_bits);
        ciphertext2 - shared_secret
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
        arithmetic::Field,
        circuit::{floor_planner::V1, Layouter},
        dev::MockProver,
        halo2curves::{bn256::Fr, grumpkin},
        plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
    };

    use super::{
        off_circuit, ElGamalEncryptionChip, ElGamalEncryptionChipOutput, ElGamalEncryptionInput,
    };
    use crate::{
        column_pool::{ColumnPool, PreSynthesisPhase},
        config_builder::ConfigsBuilder,
        curve_arithmetic::{field_element_to_le_bits, normalize_point, GrumpkinPoint},
        embed::Embed,
        generate_keys, rng,
        synthesizer::create_synthesizer,
    };

    #[derive(Clone, Debug, Default)]
    struct ElGamalEncryptionCircuit(ElGamalEncryptionInput<Fr>);

    impl Circuit<Fr> for ElGamalEncryptionCircuit {
        type Config = (
            ColumnPool<Advice, PreSynthesisPhase>,
            ElGamalEncryptionChip,
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

            let configs_builder = ConfigsBuilder::new(meta)
                .with_scalar_multiply_chip()
                .with_points_add_chip()
                .with_sum();

            let chip = ElGamalEncryptionChip {
                multiply_chip: configs_builder.scalar_multiply_chip(),
                add_chip: configs_builder.points_add_chip(),
                sum_chip: configs_builder.sum_chip(),
            };

            (configs_builder.finish(), chip, instance)
        }

        fn synthesize(
            &self,
            (column_pool, chip, instance): Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let column_pool = column_pool.start_synthesis();
            let mut synthesizer = create_synthesizer(&mut layouter, &column_pool);

            let input = self.0.embed(&mut synthesizer, "input")?;

            let ElGamalEncryptionChipOutput {
                ciphertext1,
                ciphertext2,
            } = chip.encrypt(&mut synthesizer, &input)?;

            synthesizer.constrain_instance(ciphertext1.x.cell(), instance, 0)?;
            synthesizer.constrain_instance(ciphertext1.y.cell(), instance, 1)?;
            synthesizer.constrain_instance(ciphertext1.z.cell(), instance, 2)?;

            synthesizer.constrain_instance(ciphertext2.x.cell(), instance, 3)?;
            synthesizer.constrain_instance(ciphertext2.y.cell(), instance, 4)?;
            synthesizer.constrain_instance(ciphertext2.z.cell(), instance, 5)?;

            Ok(())
        }
    }

    fn input(
        message: GrumpkinPoint<Fr>,
        public_key: GrumpkinPoint<Fr>,
        salt: grumpkin::Fr,
    ) -> ElGamalEncryptionInput<Fr> {
        ElGamalEncryptionInput {
            message,
            public_key,
            salt_le_bits: field_element_to_le_bits(salt),
        }
    }

    fn verify(
        input: ElGamalEncryptionInput<Fr>,
        expected: ElGamalEncryptionChipOutput<Fr>,
    ) -> Result<(), Vec<String>> {
        MockProver::run(
            12,
            &ElGamalEncryptionCircuit(input),
            vec![vec![
                expected.ciphertext1.x,
                expected.ciphertext1.y,
                expected.ciphertext1.z,
                expected.ciphertext2.x,
                expected.ciphertext2.y,
                expected.ciphertext2.z,
            ]],
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
    fn off_circuit_encryption_and_decryption() {
        let mut rng = rng();

        let (private_key, public_key) = generate_keys(&mut rng);
        let message = GrumpkinPoint::random(&mut rng);
        let salt = grumpkin::Fr::random(rng);

        let (ciphertext1, ciphertext2) = off_circuit::encrypt(message, public_key, salt);
        let recovered_message = off_circuit::decrypt(ciphertext1, ciphertext2, private_key);

        assert_eq!(message, normalize_point(recovered_message));
    }

    #[test]
    fn encrypt_random_message() {
        let mut rng = rng();

        let (_, public_key) = generate_keys(&mut rng);
        let message = GrumpkinPoint::random(&mut rng);
        let salt = grumpkin::Fr::random(rng);

        let (ciphertext1, ciphertext2) = off_circuit::encrypt(message, public_key, salt);

        let input = input(message, public_key, salt);
        let output = ElGamalEncryptionChipOutput {
            ciphertext1,
            ciphertext2,
        };

        assert!(verify(input, output).is_ok());
    }
}
