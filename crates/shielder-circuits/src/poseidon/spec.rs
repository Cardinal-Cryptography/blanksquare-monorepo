use alloc::vec::Vec;

use halo2_poseidon::poseidon::primitives::{generate_constants, Mds, Spec};

use crate::{
    consts::merkle_constants::{ARITY, WIDTH},
    Field, Fr,
};

#[derive(Copy, Clone, Debug)]
pub enum PoseidonSpec {}

impl Spec<Fr, WIDTH, ARITY> for PoseidonSpec {
    fn pre_rounds() -> usize {
        1
    }

    fn full_rounds() -> usize {
        8
    }

    fn partial_rounds() -> usize {
        48
    }

    fn sbox(val: Fr) -> Fr {
        val.pow_vartime([7])
    }

    fn secure_mds() -> usize {
        0
    }

    fn constants() -> (Vec<[Fr; WIDTH]>, Mds<Fr, WIDTH>, Mds<Fr, WIDTH>) {
        generate_constants::<Fr, Self, WIDTH, ARITY>()
    }
}
