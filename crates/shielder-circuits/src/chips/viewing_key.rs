use halo2_proofs::plonk::Error;

use crate::{
    consts::VIEWING_KEY_SALT,
    poseidon::circuit::{hash, PoseidonChip},
    synthesizer::Synthesizer,
    AssignedCell,
};

pub mod off_circuit {
    use crate::{consts::VIEWING_KEY_SALT, poseidon::off_circuit::hash, Fr};

    pub fn derive_viewing_key(id: Fr) -> Fr {
        hash(&[id, *VIEWING_KEY_SALT])
    }
}

#[derive(Clone, Debug)]
pub struct ViewingKeyChip {
    poseidon: PoseidonChip,
}

impl ViewingKeyChip {
    pub fn new(poseidon: PoseidonChip) -> Self {
        Self { poseidon }
    }

    pub fn derive_viewing_key(
        &self,
        synthesizer: &mut impl Synthesizer,
        id: AssignedCell,
    ) -> Result<AssignedCell, Error> {
        let salt = synthesizer.assign_constant("ViewingKey salt", *VIEWING_KEY_SALT)?;
        hash(synthesizer, self.poseidon.clone(), [id, salt])
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
        plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
    };
    use rand_core::OsRng;

    use crate::{
        chips::viewing_key::{off_circuit, ViewingKeyChip},
        column_pool::{ColumnPool, PreSynthesisPhase},
        config_builder::ConfigsBuilder,
        embed::Embed,
        synthesizer::create_synthesizer,
        Field, Fr,
    };

    #[derive(Clone, Debug, Default)]
    struct SymKeyCircuit {
        id: Fr,
    }

    impl Circuit<Fr> for SymKeyCircuit {
        type Config = (
            ColumnPool<Advice, PreSynthesisPhase>,
            ViewingKeyChip,
            Column<Instance>,
        );
        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            // Enable public input.
            let instance = meta.instance_column();
            meta.enable_equality(instance);
            // Register Poseidon.
            let configs_builder = ConfigsBuilder::new(meta).with_poseidon();
            // Create SymKey chip.
            let viewing_key_chip = ViewingKeyChip::new(configs_builder.poseidon_chip());

            (configs_builder.finish(), viewing_key_chip, instance)
        }

        fn synthesize(
            &self,
            (pool, chip, instance): Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let pool = pool.start_synthesis();
            let mut synthesizer = create_synthesizer(&mut layouter, &pool);
            // 1. Embed id.
            let id = self.id.embed(&mut synthesizer, "id")?;

            // 2. Derive viewing key.
            let viewing_key = chip.derive_viewing_key(&mut synthesizer, id)?;

            // 3. Compare with public input.
            synthesizer.constrain_instance(viewing_key.cell(), instance, 0)
        }
    }

    fn verify(id: impl Into<Fr>, expected_viewing_key: impl Into<Fr>) -> Result<(), Vec<String>> {
        MockProver::run(
            6,
            &SymKeyCircuit { id: id.into() },
            vec![vec![expected_viewing_key.into()]],
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
        let id = Fr::random(OsRng);
        let viewing_key = off_circuit::derive_viewing_key(id);
        assert!(verify(id, viewing_key).is_ok());
    }

    #[test]
    fn incorrect_viewing_key_fails() {
        let expected_viewing_key = off_circuit::derive_viewing_key(Fr::from(41));
        let another_id = Fr::from(42);

        let mut errors = verify(another_id, expected_viewing_key)
            .expect_err("Verification should fail")
            .into_iter();

        assert!(errors
            .any(|error| error
                .contains("Equality constraint not satisfied by cell (Column('Advice'")));
    }
}
