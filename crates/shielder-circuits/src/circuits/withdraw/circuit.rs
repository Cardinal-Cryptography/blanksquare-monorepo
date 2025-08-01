use halo2_proofs::{
    circuit::{floor_planner::V1, Layouter},
    plonk::{Advice, Circuit, ConstraintSystem, Error},
};

use crate::{
    circuits::withdraw::chip::WithdrawChip,
    column_pool::{ColumnPool, PreSynthesisPhase},
    config_builder::ConfigsBuilder,
    embed::Embed,
    instance_wrapper::InstanceWrapper,
    synthesizer::create_synthesizer,
    withdraw::{WithdrawInstance, WithdrawProverKnowledge},
    Fr, Value,
};

#[derive(Clone, Debug, Default)]
pub struct WithdrawCircuit(pub WithdrawProverKnowledge<Value>);

impl Circuit<Fr> for WithdrawCircuit {
    type Config = (WithdrawChip, ColumnPool<Advice, PreSynthesisPhase>);
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Default::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let public_inputs = InstanceWrapper::<WithdrawInstance>::new(meta);

        let configs_builder = ConfigsBuilder::new(meta)
            .with_merkle(public_inputs.narrow())
            .with_range_check()
            .with_note(public_inputs.narrow());

        (
            WithdrawChip {
                public_inputs,
                poseidon: configs_builder.poseidon_chip(),
                merkle: configs_builder.merkle_chip(),
                range_check: configs_builder.range_check_chip(),
                sum_chip: configs_builder.sum_chip(),
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
        let knowledge = self.0.embed(&mut synthesizer, "WithdrawProverKnowledge")?;

        main_chip.check_old_note(&mut synthesizer, &knowledge)?;
        main_chip.check_old_nullifier(&mut synthesizer, &knowledge)?;
        main_chip.check_new_note(&mut synthesizer, &knowledge)?;
        main_chip.check_commitment(&mut synthesizer, &knowledge)?;
        main_chip.check_mac(&mut synthesizer, &knowledge)
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::halo2curves::bn256::Fr;
    use rand::{rngs::SmallRng, SeedableRng};
    use rand_core::OsRng;

    use crate::{
        chips::viewing_key::off_circuit,
        circuits::{
            merkle::generate_example_path_with_given_leaf,
            test_utils::{
                expect_prover_success_and_run_verification,
                expect_prover_success_and_run_verification_on_separate_pub_input,
                run_full_pipeline, PublicInputProviderExt,
            },
            withdraw::knowledge::WithdrawProverKnowledge,
        },
        consts::merkle_constants::NOTE_TREE_HEIGHT,
        generate_keys_with_min_k, generate_proof, generate_setup_params, note_hash,
        poseidon::off_circuit::hash,
        test_utils::expect_instance_permutation_failures,
        version::NOTE_VERSION,
        withdraw::WithdrawInstance::{self, *},
        Field, Note, NoteVersion, ProverKnowledge, PublicInputProvider, MAX_K,
    };

    #[test]
    fn passes_if_inputs_correct() {
        run_full_pipeline::<WithdrawProverKnowledge<Fr>>();
    }

    #[test]
    fn passes_with_nonnative_token() {
        let mut rng = SmallRng::from_seed([42; 32]);
        let mut pk = WithdrawProverKnowledge::random_correct_example(&mut rng);

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
        hash_input[0] = pk.account_old_balance - pk.withdrawal_value;
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
        assert_eq!(Fr::from(123), pub_input[4]);
    }

    #[test]
    fn fails_if_merkle_proof_uses_wrong_note() {
        let mut pk = WithdrawProverKnowledge::random_correct_example(&mut OsRng);

        let (merkle_root, path) =
            generate_example_path_with_given_leaf(Fr::random(&mut OsRng), &mut OsRng);
        pk.path = path;
        let pub_input = pk.with_substitution(MerkleRoot, |_| merkle_root);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input,).is_err()
        );
    }

    #[test]
    fn fails_if_incorrect_h_nullifier_is_published() {
        let pk = WithdrawProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(HashedOldNullifier, |hash| hash + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_h_note_new_is_not_the_hash_of_appropriate_witnesses() {
        let pk = WithdrawProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(HashedNewNote, |hash| hash + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_mac_commitment_is_incorrect() {
        let pk = WithdrawProverKnowledge::random_correct_example(&mut OsRng);
        let pub_input = pk.with_substitution(MacCommitment, |c| c + Fr::ONE);

        assert!(
            expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input).is_err()
        );
    }

    #[test]
    fn fails_if_mac_salt_is_incorrect() {
        let pk = WithdrawProverKnowledge::random_correct_example(&mut OsRng);
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
            let mut pk = WithdrawProverKnowledge::random_correct_example(&mut rng);

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
            let account_balance_new = pk.account_old_balance - pk.withdrawal_value;

            // Build the new note.
            let h_note_new = note_hash(&Note {
                version: NOTE_VERSION,
                id: pk.id,
                nullifier: pk.nullifier_new,
                account_balance: account_balance_new,
                token_address: pk.token_address,
            });

            let pub_input = |instance: WithdrawInstance| match instance {
                MerkleRoot => merkle_root,
                HashedOldNullifier => h_nullifier_old,
                HashedNewNote => h_note_new,
                WithdrawalValue => pk.withdrawal_value,
                Commitment => pk.commitment,
                TokenAddress => pk.token_address,
                MacSalt => pk.mac_salt,
                MacCommitment => hash(&[pk.mac_salt, off_circuit::derive_viewing_key(pk.id)]),
            };

            assert_eq!(
                expect_prover_success_and_run_verification(
                    pk.create_circuit(),
                    &pub_input.serialize_public_input(),
                )
                .is_ok(),
                verify_is_expected_to_pass
            );
        }
    }

    #[test]
    fn fails_if_commitment_provided_during_verify_is_not_one_provided_during_proof() {
        let pk = WithdrawProverKnowledge::random_correct_example(&mut OsRng);

        let prove_public_input = pk.serialize_public_input();
        assert!(expect_prover_success_and_run_verification(
            pk.create_circuit(),
            &prove_public_input,
        )
        .is_ok());

        let verify_public_input = pk.with_substitution(Commitment, |c| c + Fr::ONE);
        assert!(
            expect_prover_success_and_run_verification_on_separate_pub_input(
                pk.create_circuit(),
                &prove_public_input,
                &verify_public_input,
            )
            .is_err()
        );
    }

    #[test]
    #[should_panic]
    fn fails_if_new_balance_overflowed() {
        let mut pk = WithdrawProverKnowledge::random_correct_example(&mut OsRng);

        // `F::-1` should fail the range check.
        pk.withdrawal_value = -Fr::ONE;

        let params = generate_setup_params(MAX_K, &mut OsRng);
        let circuit = pk.create_circuit();
        let (params, _, key, _) = generate_keys_with_min_k(circuit.clone(), params).unwrap();
        generate_proof(
            &params,
            &key,
            circuit,
            &pk.serialize_public_input(),
            &mut OsRng,
        );
    }

    #[test]
    fn fails_if_token_address_pub_input_incorrect() {
        let mut rng = SmallRng::from_seed([42; 32]);
        let pk = WithdrawProverKnowledge::random_correct_example(&mut rng);
        let pub_input = pk.with_substitution(TokenAddress, |v| v + Fr::ONE);

        let failures = expect_prover_success_and_run_verification(pk.create_circuit(), &pub_input)
            .expect_err("Verification must fail");

        expect_instance_permutation_failures(
            &failures,
            // The returned failure location happens to be in
            // a `poseidon-gadget` region the token address was copied to.
            "add input for domain ConstantLength<7>",
            4,
        );
    }

    // TODO: Add more tests, as the above tests do not cover all the logic that should be covered.
}
