use halo2_proofs::plonk::Error;
use strum_macros::{EnumCount, EnumIter};

use crate::{
    instance_wrapper::InstanceWrapper,
    poseidon::circuit::{hash, PoseidonChip},
    synthesizer::Synthesizer,
    AssignedCell,
};

/// Input for MAC calculation.
#[derive(Copy, Clone, Debug, Default)]
pub struct MacInput<T> {
    pub key: T,
    pub salt: T,
}

/// MAC (commitment to a key accompanied by salt).
#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub struct Mac<T> {
    pub salt: T,
    pub commitment: T,
}

#[allow(dead_code)]
pub mod off_circuit {
    use crate::{
        chips::mac::{Mac, MacInput},
        poseidon::off_circuit::hash,
        Fr,
    };

    pub fn mac(input: &MacInput<Fr>) -> Mac<Fr> {
        Mac {
            salt: input.salt,
            commitment: hash(&[input.salt, input.key]),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, EnumIter, EnumCount)]
pub enum MacInstance {
    MacSalt,
    MacCommitment,
}

/// Chip that is able to calculate MAC.
///
/// Given a key `key` and a random value `r`, MAC is calculated as `(r, H(r, key))`.
#[derive(Clone, Debug)]
pub struct MacChip {
    poseidon: PoseidonChip,
    instance: InstanceWrapper<MacInstance>,
}

impl MacChip {
    /// Create a new `MacChip`.
    pub fn new(poseidon: PoseidonChip, instance: InstanceWrapper<MacInstance>) -> Self {
        Self { poseidon, instance }
    }

    /// Calculate the MAC as `(r, H(r, key))`.
    pub fn mac(
        &self,
        synthesizer: &mut impl Synthesizer,
        input: &MacInput<AssignedCell>,
    ) -> Result<(), Error> {
        let commitment = hash(
            synthesizer,
            self.poseidon.clone(),
            [input.salt.clone(), input.key.clone()],
        )?;

        self.instance.constrain_cells(
            synthesizer,
            [
                (input.salt.clone(), MacInstance::MacSalt),
                (commitment, MacInstance::MacCommitment),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    use halo2_proofs::{
        circuit::{floor_planner::V1, Layouter},
        dev::MockProver,
        plonk::{Advice, Circuit, ConstraintSystem, Error},
    };

    use crate::{
        chips::mac::{off_circuit, Mac, MacChip, MacInput},
        column_pool::{ColumnPool, PreSynthesisPhase},
        config_builder::ConfigsBuilder,
        embed::Embed,
        instance_wrapper::InstanceWrapper,
        synthesizer::create_synthesizer,
        Fr,
    };

    #[derive(Clone, Debug, Default)]
    struct MacCircuit(MacInput<Fr>);

    impl Circuit<Fr> for MacCircuit {
        type Config = (ColumnPool<Advice, PreSynthesisPhase>, MacChip);
        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            let instance = InstanceWrapper::new(meta);
            let configs_builder = ConfigsBuilder::new(meta).with_poseidon();
            let mac = MacChip::new(configs_builder.poseidon_chip(), instance);

            (configs_builder.finish(), mac)
        }

        fn synthesize(
            &self,
            (pool, mac_chip): Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let pool = pool.start_synthesis();
            let mut synthesizer = create_synthesizer(&mut layouter, &pool);
            // 1. Embed key and salt.
            let key = self.0.key.embed(&mut synthesizer, "key")?;
            let salt = self.0.salt.embed(&mut synthesizer, "salt")?;

            // 2. Compute MAC and constrain the result to the instance.
            mac_chip.mac(&mut synthesizer, &MacInput { key, salt })
        }
    }

    fn input(key: impl Into<Fr>, salt: impl Into<Fr>) -> MacInput<Fr> {
        MacInput {
            key: key.into(),
            salt: salt.into(),
        }
    }

    fn verify(input: MacInput<Fr>, expected_mac: Mac<Fr>) -> Result<(), Vec<String>> {
        MockProver::run(
            6,
            &MacCircuit(input),
            vec![vec![expected_mac.salt, expected_mac.commitment]],
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
    fn correct_input_passes() {
        let input = input(41, 42);
        let mac = off_circuit::mac(&input);

        assert!(verify(input, mac).is_ok());
    }

    #[test]
    fn incorrect_mac_fails() {
        let expected_mac = off_circuit::mac(&input(1, 42));
        let input = input(41, 42);

        let mut errors = verify(input, expected_mac)
            .expect_err("Verification should fail")
            .into_iter();

        assert!(errors
            .any(|error| error
                .contains("Equality constraint not satisfied by cell (Column('Advice'")));
    }

    #[test]
    fn incorrect_salt_fails() {
        let mut expected_mac = off_circuit::mac(&input(41, 42));
        expected_mac.salt += Fr::one();
        let input = input(41, 42);

        let mut errors = verify(input, expected_mac)
            .expect_err("Verification should fail")
            .into_iter();

        assert!(errors
            .any(|error| error
                .contains("Equality constraint not satisfied by cell (Column('Advice'")));
    }
}
