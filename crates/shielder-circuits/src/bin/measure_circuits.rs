use halo2_proofs::dev::CircuitCost;
use human_bytes::human_bytes;
use once_cell::sync::Lazy;
use rand::rngs::StdRng;
use rand_core::{OsRng, SeedableRng};
use shielder_circuits::{
    circuits::{
        deposit::DepositProverKnowledge, merkle::MerkleProverKnowledge,
        new_account::NewAccountProverKnowledge, withdraw::WithdrawProverKnowledge, Params,
    },
    consts::merkle_constants::NOTE_TREE_HEIGHT,
    generate_keys_with_min_k, generate_proof, generate_setup_params, Fr, ProverKnowledge, G1,
    MAX_K, SERDE_FORMAT,
};

static PARAMS: Lazy<Params> =
    Lazy::new(|| generate_setup_params(MAX_K, &mut StdRng::from_seed(Default::default())));

fn measure_circuit<PK: ProverKnowledge>(circuit_name: &str) {
    let values = PK::random_correct_example(&mut rand::thread_rng());
    let circuit = values.create_circuit();
    let (reduced_params, k, pk, vk) =
        generate_keys_with_min_k(PK::Circuit::default(), PARAMS.clone()).unwrap();
    let cost = CircuitCost::<G1, _>::measure(k, &circuit);

    let proof = generate_proof(
        &reduced_params,
        &pk,
        circuit,
        &values.serialize_public_input(),
        &mut OsRng,
    );

    println!(
        "`{circuit_name}` proof size:            {}",
        human_bytes(proof.len() as f64)
    );
    println!(
        "`{circuit_name}` proving key size:      {}",
        human_bytes(pk.to_bytes(SERDE_FORMAT).len() as f64)
    );
    println!(
        "`{circuit_name}` verification key size: {}",
        human_bytes(vk.to_bytes(SERDE_FORMAT).len() as f64)
    );

    let cost = format!("\n{:#?}", cost);
    println!(
        "{}\n",
        cost.replace("\n", &format!("\n`{}` ", circuit_name)).trim()
    );
}

fn main() {
    measure_circuit::<NewAccountProverKnowledge<Fr>>("New account");
    measure_circuit::<DepositProverKnowledge<Fr>>("Deposit");
    measure_circuit::<WithdrawProverKnowledge<Fr>>("Withdraw");
    measure_circuit::<MerkleProverKnowledge<NOTE_TREE_HEIGHT, Fr>>("Merkle");
}
