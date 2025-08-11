use halo2_proofs::{
    circuit::{floor_planner::V1, Layouter},
    plonk::{Advice, Circuit, ConstraintSystem, Error},
};

use crate::{
    circuits::deposit::{chip::DepositChip, knowledge::DepositProverKnowledge},
    column_pool::{ColumnPool, PreSynthesisPhase},
    config_builder::ConfigsBuilder,
    deposit::DepositInstance,
    embed::Embed,
    instance_wrapper::InstanceWrapper,
    synthesizer::create_synthesizer,
    Fr, Value,
};

#[derive(Clone, Debug, Default)]
pub struct DepositCircuit(pub DepositProverKnowledge<Value>);

impl Circuit<Fr> for DepositCircuit {
    type Config = (DepositChip, ColumnPool<Advice, PreSynthesisPhase>);
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let public_inputs = InstanceWrapper::<DepositInstance>::new(meta);

        let configs_builder = ConfigsBuilder::new(meta)
            .with_poseidon()
            .with_merkle(public_inputs.narrow())
            .with_note(public_inputs.narrow());

        (
            DepositChip {
                public_inputs,
                poseidon: configs_builder.poseidon_chip(),
                merkle: configs_builder.merkle_chip(),
                note: configs_builder.note_chip(),
            },
            configs_builder.finish(),
        )
    }

    fn synthesize(
        &self,
        (main_chip, column_pool): Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        let pool = column_pool.start_synthesis();
        let mut synthesizer = create_synthesizer(&mut layouter, &pool);
        let knowledge = self.0.embed(&mut synthesizer, "DepositProverKnowledge")?;

        main_chip.check_old_note(&mut synthesizer, &knowledge)?;
        main_chip.check_old_nullifier(&mut synthesizer, &knowledge)?;
        main_chip.check_new_note(&mut synthesizer, &knowledge)?;
        main_chip.check_mac(&mut synthesizer, &knowledge)?;
        main_chip.check_commitment(&mut synthesizer, &knowledge)
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{arithmetic::Field, halo2curves::bn256::Fr};
    use rand::{rngs::SmallRng, SeedableRng};
    use rand_core::OsRng;

    use crate::{
        chips::viewing_key::off_circuit,
        circuits::{
            deposit::knowledge::DepositProverKnowledge,
            merkle::generate_example_path_with_given_leaf,
            test_utils::{
                expect_prover_success_and_run_verification, run_full_pipeline,
                PublicInputProviderExt,
            },
        },
        consts::merkle_constants::NOTE_TREE_HEIGHT,
        deposit::DepositInstance::{self, *},
        note_hash,
        poseidon::off_circuit::hash,
        test_utils::expect_instance_permutation_failures,
        version::NOTE_VERSION,
        Note, NoteVersion, ProverKnowledge, PublicInputProvider,
    };

    #[test]
    fn passes_if_inputs_correct() {
        run_full_pipeline::<DepositProverKnowledge<Fr>>();
    }

    #[test]
    fn fails_if_merkle_proof_uses_wrong_note() {
        let mut pk = DepositProverKnowledge::random_correct_example(&mut OsRng);

        let (merkle_root, path) =
            generate_example_path_with_given_leaf(Fr::random(&mut OsRng), &mut OsRng);
        pk.path = path;
        let pub_input = pk.with_substitution(MerkleRoot, |_| merkle_root);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_incorrect_h_nullifier_is_published() {
        let pk = DepositProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(HashedOldNullifier, |hash| hash + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_h_note_new_is_not_the_hash_of_appropriate_witnesses() {
        let pk = DepositProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(HashedNewNote, |hash| hash + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_mac_commitment_is_incorrect() {
        let pk = DepositProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(MacCommitment, |c| c + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_mac_salt_is_incorrect() {
        let pk = DepositProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(MacSalt, |s| s + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_h_note_old_is_not_the_hash_of_appropriate_witnesses() {
        let mut rng = OsRng;

        // First we break the witness and expect verification failure, then we run with correct
        // witnesses and expect verification success. If we just did the former, it would be easy
        // to introduce a bug to this test and the test would pass without checking anything.
        for (modification, verify_is_expected_to_pass) in [(Fr::ONE, false), (Fr::ZERO, true)] {
            let mut pk = DepositProverKnowledge::random_correct_example(&mut rng);

            // Build the old note.
            let h_note_old = note_hash(&Note {
                version: NOTE_VERSION,
                id: pk.id,
                nullifier: pk.nullifier_old,
                account_balance: pk.account_old_balance,
                token_address: pk.token_address,
            }) + modification /* Modification here! */;
            let h_nullifier_old = hash(&[pk.nullifier_old]);

            // Build the Merkle proof.
            let (merkle_root, path) = generate_example_path_with_given_leaf(h_note_old, &mut rng);
            pk.path = path;

            // Build the new account state.
            let account_balance_new = pk.account_old_balance + pk.deposit_value;

            // Build the new note.
            let h_note_new = note_hash(&Note {
                version: NOTE_VERSION,
                id: pk.id,
                nullifier: pk.nullifier_new,
                account_balance: account_balance_new,
                token_address: pk.token_address,
            });

            let pub_input = |instance: DepositInstance| match instance {
                MerkleRoot => merkle_root,
                HashedOldNullifier => h_nullifier_old,
                HashedNewNote => h_note_new,
                // Important note: there is no range check in the circuit for DepositValue, however there is an external constraint
                // (in the smart contract) guaranteeing that this never exceeds MAX_CONTRACT_BALANCE = 2^{112} - 1.
                DepositValue => pk.deposit_value,
                Commitment => pk.commitment,
                TokenAddress => pk.token_address,
                MacSalt => pk.mac_salt,
                MacCommitment => hash(&[pk.mac_salt, off_circuit::derive_viewing_key(pk.id)]),
            };

            assert_eq!(
                expect_prover_success_and_run_verification(
                    pk.create_circuit(),
                    &pub_input.serialize_public_input()
                )
                .is_ok(),
                verify_is_expected_to_pass
            );
        }
    }

    #[test]
    fn passes_with_nonnative_token() {
        let mut rng = SmallRng::from_seed([42; 32]);
        let mut pk = DepositProverKnowledge::random_correct_example(&mut rng);

        pk.token_address = Fr::from(123);

        // Substitute all that changes in `pk` when `token_address` changes.
        let h_note_old = note_hash(&Note {
            version: NoteVersion::new(0),
            id: pk.id,
            nullifier: pk.nullifier_old,
            account_balance: pk.account_old_balance,
            token_address: pk.token_address,
        });
        let (_, path) =
            generate_example_path_with_given_leaf::<NOTE_TREE_HEIGHT>(h_note_old, &mut rng);
        pk.path = path;

        let pub_input = pk.serialize_public_input();

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_ok()
        );

        // Manually verify that the new note is as expected.
        let mut hash_input = [Fr::ZERO; 7];
        hash_input[0] = pk.account_old_balance + pk.deposit_value;
        hash_input[1] = pk.token_address;
        let new_balance_hash = hash(&hash_input);
        let new_note_hash = hash(&[
            Fr::ZERO, // Note version.
            pk.id,
            pk.nullifier_new,
            new_balance_hash,
        ]);
        assert_eq!(new_note_hash, pub_input[2]);

        // Verify the token address.
        assert_eq!(Fr::from(123), pub_input[5]);
    }

    #[test]
    fn fails_if_token_address_pub_input_incorrect() {
        let mut rng = SmallRng::from_seed([42; 32]);
        let pk = DepositProverKnowledge::random_correct_example(&mut rng);
        let pub_input = pk.with_substitution(TokenAddress, |v| v + Fr::ONE);

        let failures = expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input)
            .expect_err("Verification must fail");

        expect_instance_permutation_failures(
            &failures,
            // The returned failure location happens to be in
            // a `poseidon-gadget` region the token address was copied to.
            "add input for domain ConstantLength<7>",
            5,
        );
    }

    #[test]
    fn fails_if_commitment_is_incorrect() {
        let pk = DepositProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(Commitment, |s| s + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    // TODO: Add more tests, as the above tests do not cover all the logic that should be covered.
}
