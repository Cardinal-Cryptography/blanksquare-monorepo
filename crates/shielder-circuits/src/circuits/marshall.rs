use alloc::{vec, vec::Vec};
use core::fmt::{self, Display, Formatter};

use halo2_proofs::{halo2curves::serde::SerdeObject, plonk::Circuit};

use crate::{
    circuits::{Params, ProvingKey},
    consts::merkle_constants::{ARITY, NOTE_TREE_HEIGHT},
    marshall::MarshallError::{InvalidContent, IoError},
    Fr, SERDE_FORMAT,
};

#[derive(Debug)]
pub enum MarshallError {
    IoError,
    InvalidContent,
}

impl Display for MarshallError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            IoError => write!(f, "IO error"),
            InvalidContent => write!(f, "Invalid content. Couldn't parse the data."),
        }
    }
}

pub type MarshallResult<T> = Result<T, MarshallError>;

/// Serialize `params` to bytes.
pub fn marshall_params(params: &Params) -> MarshallResult<Vec<u8>> {
    let mut buf = vec![];
    params
        .write_custom(&mut buf, SERDE_FORMAT)
        .map_err(|_| IoError)?;
    Ok(buf)
}

/// Deserialize `params` from bytes.
pub fn unmarshall_params(mut buf: &[u8]) -> MarshallResult<Params> {
    Params::read_custom(&mut buf, SERDE_FORMAT).map_err(|_| IoError)
}

/// Serialize `pk` to bytes together with `k` - minimal sufficient number of rows (log2 of it).
pub fn marshall_pk(k: u32, pk: &ProvingKey) -> Vec<u8> {
    [k.to_be_bytes().to_vec(), pk.to_bytes(SERDE_FORMAT)].concat()
}

/// Deserialize `pk` from bytes together with `k`. `k` can be then used to downsize parameters.
pub fn unmarshall_pk<C: Circuit<Fr> + Default>(buf: &[u8]) -> MarshallResult<(u32, ProvingKey)> {
    let k = u32::from_be_bytes(buf[..4].try_into().map_err(|_| InvalidContent)?);
    ProvingKey::read::<_, C>(&mut &buf[4..], SERDE_FORMAT)
        .map_err(|_| IoError)
        .map(|pk| (k, pk))
}

/// Serialize `(leaf, path)` to bytes.
pub fn marshall_path(leaf: &Fr, path: &[[Fr; ARITY]; NOTE_TREE_HEIGHT]) -> Vec<u8> {
    let mut buf = vec![];
    leaf.write_raw(&mut buf).expect("leaf should serialize");
    for level in path.iter() {
        for node in level.iter() {
            node.write_raw(&mut buf).expect("node should serialize");
        }
    }
    buf
}

/// Deserialize `(root, leaf, path)` from bytes.
pub fn unmarshall_path(mut buf: &[u8]) -> (Fr, [[Fr; ARITY]; NOTE_TREE_HEIGHT]) {
    let leaf = Fr::read_raw(&mut buf).expect("leaf should deserialize");
    let mut path = [[Fr::default(); ARITY]; NOTE_TREE_HEIGHT];
    for level in path.iter_mut() {
        for node in level.iter_mut() {
            *node = Fr::read_raw(&mut buf).expect("node should deserialize");
        }
    }
    (leaf, path)
}

#[cfg(test)]
mod tests {
    use std::format;

    use crate::{
        circuits::{
            generate_keys_with_min_k, generate_setup_params,
            merkle::{MerkleCircuit, MerkleProverKnowledge},
        },
        consts::MAX_K,
        marshall::*,
        Fr, ProverKnowledge,
    };

    fn generate_data() -> (Params, u32, ProvingKey) {
        let mut rng = rand::thread_rng();
        let (params, k, pk, _) = generate_keys_with_min_k(
            MerkleCircuit::<NOTE_TREE_HEIGHT>::default(),
            generate_setup_params(MAX_K, &mut rng),
        )
        .expect("keys should not fail to generate");
        (params, k, pk)
    }

    #[test]
    fn marshalling_params() {
        let (params, _, _) = generate_data();

        let bytes = marshall_params(&params).unwrap();
        let params2 = unmarshall_params(&bytes).unwrap();

        assert_eq!(format!("{params:?}"), format!("{params2:?}"));
    }

    #[test]
    fn marshalling_pk() {
        let (_, k, pk) = generate_data();

        let bytes = marshall_pk(k, &pk);
        let (k2, pk2) = unmarshall_pk::<MerkleCircuit<NOTE_TREE_HEIGHT>>(&bytes).unwrap();

        assert_eq!(k, k2);
        assert_eq!(format!("{pk:?}"), format!("{pk2:?}"));
    }

    #[test]
    fn marshalling_path() {
        let mut rng = rand::thread_rng();

        let merkle_prover_knowledge =
            MerkleProverKnowledge::<NOTE_TREE_HEIGHT, Fr>::random_correct_example(&mut rng);

        let bytes = marshall_path(&merkle_prover_knowledge.leaf, &merkle_prover_knowledge.path);
        let (leaf2, path2) = unmarshall_path(&bytes);

        assert_eq!(
            (merkle_prover_knowledge.leaf, merkle_prover_knowledge.path),
            (leaf2, path2)
        );
    }
}
