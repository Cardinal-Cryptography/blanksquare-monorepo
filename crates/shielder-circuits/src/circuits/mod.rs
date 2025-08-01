use alloc::{vec, vec::Vec};

use halo2_proofs::{
    dev::MockProver,
    halo2curves::bn256::{Bn256, Fr, G1Affine},
    plonk::{create_proof, keygen_pk, keygen_vk_custom, verify_proof, Circuit, Error},
    poly::{
        commitment::{Params as _, ParamsProver},
        kzg::{
            commitment::{KZGCommitmentScheme, ParamsKZG},
            multiopen::{ProverSHPLONK, VerifierSHPLONK},
            strategy::SingleStrategy,
        },
    },
    transcript::TranscriptWriterBuffer as _,
};
use rand_core::RngCore;
use transcript::Keccak256Transcript;

use crate::consts::MAX_K;

pub mod deposit;
pub mod merkle;
pub mod new_account;
pub mod withdraw;

pub mod marshall;
#[cfg(test)]
pub mod test_utils;
#[cfg(test)]
pub use test_utils::rng;

pub type Params = ParamsKZG<Bn256>;
pub type ProvingKey = halo2_proofs::plonk::ProvingKey<G1Affine>;
pub type VerifyingKey = halo2_proofs::plonk::VerifyingKey<G1Affine>;
pub type CommitmentScheme = KZGCommitmentScheme<Bn256>;
pub type Prover<'a> = ProverSHPLONK<'a, Bn256>;
pub type Verifier<'a> = VerifierSHPLONK<'a, Bn256>;

pub const COMPRESS_SELECTORS: bool = true;

// Generates setup parameters with given `k`. This restricts the circuit to at most `2^k` rows.
pub fn generate_setup_params<R: RngCore>(k: u32, rng: &mut R) -> Params {
    Params::setup(k, rng)
}

// Generates the verifying and proving keys. Downsizes `k` in `params` to the smallest value
// for which key generation succeeds. The passed `circuit` is allowed to be empty.
//
// Returns modified `params`, minimal `k`, and both the keys, or an error if no valid `k` is found.
pub fn generate_keys_with_min_k(
    circuit: impl Circuit<Fr>,
    params: Params,
) -> Result<(Params, u32, ProvingKey, VerifyingKey), Error> {
    let circuit = circuit.without_witnesses();
    let mut last_err = None;

    for k in 6..MAX_K {
        let mut params = params.clone();
        params.downsize(k);
        match keygen_vk_custom(&params, &circuit, COMPRESS_SELECTORS) {
            Ok(vk) => {
                let pk = keygen_pk(&params, vk.clone(), &circuit)
                    .expect("pk generation should not fail");
                return Ok((params, k, pk, vk));
            }
            Err(e) => last_err = Some(e),
        }
    }

    Err(last_err.expect("Loop has failed at least once"))
}

// Runs the mock prover and panics in case of an error.
pub fn run_mock_prover<C: Circuit<Fr>>(k: u32, circuit: &C, pub_input: Vec<Fr>) {
    let prover = MockProver::run(k, circuit, vec![pub_input]).expect("Mock prover should run");
    let res = prover.verify();
    match res {
        Ok(()) => (),
        Err(e) => panic!("Circuit not satisfied: {:?}", e),
    }
}

pub fn generate_proof<C: Circuit<Fr>>(
    params: &Params,
    pk: &ProvingKey,
    circuit: C,
    pub_input: &[Fr],
    rng: &mut impl RngCore,
) -> Vec<u8> {
    let mut transcript = Keccak256Transcript::new(Vec::new());

    create_proof::<CommitmentScheme, Prover, _, _, _, C>(
        params,
        pk,
        &[circuit],
        &[&[pub_input]],
        rng,
        &mut transcript,
    )
    .expect("proof should not fail to generate");

    transcript.finalize().to_vec()
}

pub fn verify(
    params: &Params,
    vk: &VerifyingKey,
    transcript: &[u8],
    instance: &[Fr],
) -> Result<(), Error> {
    let mut transcript = Keccak256Transcript::new(transcript);

    verify_proof::<CommitmentScheme, Verifier, _, _, _>(
        params.verifier_params(),
        vk,
        SingleStrategy::new(params.verifier_params()),
        &[&[instance]],
        &mut transcript,
    )
}
