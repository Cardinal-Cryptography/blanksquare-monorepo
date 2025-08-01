//! Helpers to be used in unit tests. These helpers are adjusted
//! not for efficiency but for ease of use.

use std::{format, string::ToString, vec, vec::Vec};

use halo2_proofs::{
    dev::{FailureLocation, MockProver, VerifyFailure},
    halo2curves::bn256::Fr,
    plonk::{Any, Circuit},
};
use rand::{rngs::StdRng, SeedableRng};
use rand_core::OsRng;
use regex::Regex;
use strum::{EnumCount, IntoEnumIterator};

use crate::{
    circuits::{self, generate_keys_with_min_k, generate_setup_params, verify},
    consts::MAX_K,
    generate_proof, ProverKnowledge, PublicInputProvider,
};

pub trait PublicInputProviderExt<Id: IntoEnumIterator + EnumCount + PartialEq>:
    PublicInputProvider<Id>
{
    fn with_substitution(&self, id: Id, change: impl Fn(Fr) -> Fr) -> Vec<Fr> {
        Id::iter()
            .map(|instance_id| {
                if instance_id == id {
                    change(self.compute_public_input(instance_id))
                } else {
                    self.compute_public_input(instance_id)
                }
            })
            .collect()
    }
}

impl<Id: IntoEnumIterator + EnumCount + PartialEq, PIP: PublicInputProvider<Id>>
    PublicInputProviderExt<Id> for PIP
{
}

/// Runs a full pipeline for a circuit that should succeed. This includes:
///  - creating circuit from a correct example,
///  - running a mock prover,
///  - generating keys and proof,
///  - verifying the proof.
pub fn run_full_pipeline<PK: ProverKnowledge>() {
    let mut rng = OsRng;

    let prover_knowledge = PK::random_correct_example(&mut rng);
    let circuit = prover_knowledge.create_circuit();
    let pub_input = prover_knowledge.serialize_public_input();

    // In case of failure, this will fail promptly and output useful debug info.
    run_mock_prover(&circuit, &pub_input);
    // Run full heavy pipeline.
    expect_prover_success_and_run_verification(circuit, &pub_input)
        .expect("Proving-verifying pipeline should succeed");
}

// Runs key generation, proof production on `prove_pub_input`, and proof verification
// on `verify_pub_input`. In case of failure before verification, panics.
// In case of verification failure, returns `VerifyFailure`s from `MockProver`.
pub fn expect_prover_success_and_run_verification_on_separate_pub_input(
    test_circuit: impl Circuit<Fr> + Clone,
    prove_pub_input: &[Fr],
    verify_pub_input: &[Fr],
) -> Result<(), Vec<VerifyFailure>> {
    let mut rng = OsRng;

    let params = generate_setup_params(MAX_K, &mut rng);

    let (params, k, pk, vk) = generate_keys_with_min_k(test_circuit.clone(), params)
        .expect("key generation should succeed");

    let proof = generate_proof(
        &params,
        &pk,
        test_circuit.clone(),
        prove_pub_input,
        &mut rng,
    );

    verify(&params, &vk, &proof, verify_pub_input).map_err(|_| {
        // Replace the verification error with actual failures from the mock prover.
        let prover = MockProver::run(k, &test_circuit, vec![verify_pub_input.to_vec()])
            .expect("Mock prover should run");
        prover
            .verify()
            .expect_err("Mock prover verification should fail")
    })
}

// Runs key generation, proof production, and proof verification.
// In case of failure before verification, panics.
// In case of verification failure, returns `VerifyFailure`s from `MockProver`.
pub fn expect_prover_success_and_run_verification<C>(
    test_circuit: C,
    pub_input: &[Fr],
) -> Result<(), Vec<VerifyFailure>>
where
    C: Circuit<Fr> + Clone,
{
    expect_prover_success_and_run_verification_on_separate_pub_input(
        test_circuit,
        pub_input,
        pub_input,
    )
}

// A prover that outputs useful debug info in case of failing constraints.
pub fn run_mock_prover<C: Circuit<Fr> + Clone>(test_circuit: &C, pub_input: &[Fr]) {
    let params = generate_setup_params(MAX_K, &mut OsRng);

    let (_, k, _, _) = generate_keys_with_min_k(test_circuit.clone(), params)
        .expect("key generation must succeed");

    circuits::run_mock_prover(k, test_circuit, pub_input.to_vec())
}

// Asserts that the given failure is a gate constraint failure
// with `expected_gate_name` as the gate name.
pub fn expect_gate_failure(actual: &VerifyFailure, expected_gate_name: &'static str) {
    match actual {
        VerifyFailure::ConstraintNotSatisfied { constraint, .. } => {
            // Could match, for example: Constraint 0 in gate 7 ('Gate name')
            let pattern = format!(
                r"Constraint (\d+) in gate (\d+) \('{}'\)",
                expected_gate_name
            );

            assert!(Regex::new(&pattern)
                .unwrap()
                .is_match(&constraint.to_string()));
        }
        _ => panic!("Unexpected error"),
    }
}

// Asserts that the `Vec<VerifyFailure>` is as expected for a failed public input constraint, i.e.:
//  - exactly 2 failures, 1 for advice and 1 for instance,
//  - `expected_advice_region_name` is present in the advice `FailureLocation`,
//  - `expected_instance_row` is present in the instance `FailureLocation`.
pub fn expect_instance_permutation_failures(
    actual: &Vec<VerifyFailure>,
    expected_advice_region_name: &str,
    expected_instance_row: usize,
) {
    assert!(actual.len() == 2);

    let mut matched_advice = false;
    let mut matched_instance = false;

    // Matches, for example: Region 123 ('Expected region name')
    let in_region_regex = Regex::new(&format!(
        r"Region \d+ \('{}'\)",
        expected_advice_region_name
    ))
    .unwrap();

    for failure in actual {
        match failure {
            VerifyFailure::Permutation { column, location } => match column.column_type() {
                Any::Advice(_) => match location {
                    FailureLocation::InRegion { region, offset: _ } => {
                        matched_advice = in_region_regex.is_match(&region.to_string())
                    }
                    _ => panic!("Unexpected failure location"),
                },
                Any::Instance => match location {
                    FailureLocation::OutsideRegion { row } => {
                        matched_instance = *row == expected_instance_row
                    }
                    _ => panic!("Unexpected failure location"),
                },
                _ => panic!("Unexpected column type"),
            },
            _ => panic!("Unexpected failure type"),
        }
    }
    assert!(matched_advice, "Advice failure not found");
    assert!(matched_instance, "Instance failure not found");
}

/// Returns an instance of rng, seeded
pub fn rng() -> StdRng {
    StdRng::from_seed(*b"00000000000000000000100001011001")
}
