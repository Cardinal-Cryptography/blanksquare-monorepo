use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use halo2_proofs::halo2curves::bn256::{Fr, G1};
use rand_core::OsRng;
use shielder_circuits::{
    circuits::{
        deposit::DepositProverKnowledge, merkle::MerkleProverKnowledge,
        new_account::NewAccountProverKnowledge, withdraw::WithdrawProverKnowledge,
    },
    consts::merkle_constants::{ARITY, NOTE_TREE_HEIGHT, WIDTH},
    generate_keys_with_min_k, generate_proof, generate_setup_params, verify, CircuitCost,
    ProverKnowledge, MAX_K,
};

pub fn bench_circuit<PK: ProverKnowledge>(c: &mut Criterion, group_name: &str) {
    let mut group = c.benchmark_group(group_name);

    let mut rng = OsRng;

    let prover_knowledge = PK::random_correct_example(&mut rng);
    let test_circuit = prover_knowledge.create_circuit();

    let params = generate_setup_params(MAX_K, &mut rng);

    let (params, k, pk, vk) = generate_keys_with_min_k(test_circuit.clone(), params)
        .expect("keys should not fail to generate");

    println!(
        "{:?}",
        CircuitCost::<G1, _>::measure(k, &test_circuit.clone())
    );

    let proof = generate_proof(
        &params.clone(),
        &pk.clone(),
        test_circuit.clone(),
        &prover_knowledge.serialize_public_input(),
        &mut rng,
    );

    // Using a closure and cloning to avoid accidentally
    // benchmarking the setup code and mutating the circuit
    // multiple times
    // TODO: Not needed right now, but keeping this pattern for future reference
    let mut do_benchmark = {
        || {
            black_box(generate_proof(
                &params.clone(),
                &pk.clone(),
                test_circuit.clone(),
                &prover_knowledge.serialize_public_input(),
                &mut rng,
            ));
        }
    };

    let multithreading = if cfg!(feature = "multithreading") {
        "yes"
    } else {
        "no"
    };

    group.bench_function(
        BenchmarkId::new(
            "prove",
            format!(
                "k={}, width={}, arity={}, height={}, multithreading={}",
                k, WIDTH, ARITY, NOTE_TREE_HEIGHT, multithreading
            ),
        ),
        |b| b.iter(&mut do_benchmark),
    );

    let do_benchmark = {
        || {
            black_box(
                verify(
                    &params.clone(),
                    &vk.clone(),
                    &proof.clone(),
                    &prover_knowledge.serialize_public_input(),
                )
                .is_ok(),
            );
        }
    };

    group.bench_function(
        BenchmarkId::new(
            "verify",
            format!(
                "k={}, width={}, arity={}, height={}, multithreading={}",
                k, WIDTH, ARITY, NOTE_TREE_HEIGHT, multithreading
            ),
        ),
        |b| b.iter(&do_benchmark),
    );
}

pub fn bench_merkle(c: &mut Criterion) {
    bench_circuit::<MerkleProverKnowledge<NOTE_TREE_HEIGHT, Fr>>(c, "MerkleCircuit")
}

criterion_group! {
    name = merkle;
    config = Criterion::default().sample_size(10);
    targets = bench_merkle
}

pub fn bench_deposit(c: &mut Criterion) {
    bench_circuit::<DepositProverKnowledge<Fr>>(c, "NoteDepositCircuit")
}

criterion_group! {
    name = deposit;
    config = Criterion::default().sample_size(10);
    targets = bench_deposit
}

pub fn bench_new_account(c: &mut Criterion) {
    bench_circuit::<NewAccountProverKnowledge<Fr>>(c, "NewAccountCircuit")
}

criterion_group! {
    name = new_account;
    config = Criterion::default().sample_size(10);
    targets = bench_new_account
}

pub fn bench_withdraw(c: &mut Criterion) {
    bench_circuit::<WithdrawProverKnowledge<Fr>>(c, "NoteWithdrawCircuit")
}

criterion_group! {
    name = withdraw;
    config = Criterion::default().sample_size(10);
    targets = bench_withdraw
}

criterion_main! {
    merkle, deposit, new_account, withdraw
}
