use std::array;

use alloy_primitives::{Address, Bytes, U256};
use shielder_account::call_data::{WithdrawCall, WithdrawCallType, WithdrawExtra};
use shielder_account::{ShielderAccount, Token};
use shielder_circuits::circuits::{Params, ProvingKey};
use shielder_circuits::{generate_proof, ProverKnowledge, PublicInputProvider};
use shielder_circuits::marshall::{self, unmarshall_params};
use shielder_circuits::withdraw::WithdrawProverKnowledge;

use rand::RngCore;
use shielder_scheduler_common::vsock::VsockError;
use shielder_setup::consts::{ARITY, TREE_HEIGHT};
use shielder_setup::version::contract_version;


#[derive(Clone, Debug)]
pub struct WithdrawCircuit {
    params: Params,
    pk: ProvingKey,
    token: Token,

    shielder_account: ShielderAccount,
}

impl WithdrawCircuit {
    pub fn new(id_seed: U256, token: Token) -> Self {
        WithdrawCircuit {
            params: Self::get_params(
                include_bytes!("../artifacts/withdraw/params.bin"),
            ),
            pk: Self::get_pk(include_bytes!("../artifacts/withdraw/pk.bin")),
            shielder_account: ShielderAccount::new(id_seed, token),
            token
        }
    }

    fn get_params(params_buf: &[u8]) -> Params {
        unmarshall_params(params_buf)
            .expect("Failed to unmarshall params")
    }

    fn get_pk(pk_buf: &[u8]) -> ProvingKey {
        let (k, pk) = marshall::unmarshall_pk::<
            <WithdrawProverKnowledge<shielder_circuits::Fr> as shielder_circuits::ProverKnowledge>::Circuit,
        >(pk_buf)
        .expect("Failed to unmarshall pk");

        pk
    }

    pub fn prove(&self, values: &WithdrawProverKnowledge<shielder_circuits::Fr>, rng: &mut impl RngCore) -> Vec<u8> {
        generate_proof(
            &self.params,
            &self.pk,
            values.create_circuit(),
            &values.serialize_public_input(),
            rng,
        )
    }

    fn get_salt() -> U256 {
	    U256::from_limbs(array::from_fn(|_| rand::random()))
    }

    pub fn get_calldata(&self, amount: U256, 
        to: Address, 
        merkle_path: Box<[[U256; ARITY]; TREE_HEIGHT]>, 
        chain_id: u64, 
        total_cost_fee_token: U256, 
        pocket_money: U256, 
        relayer_fee_address: Address,
        protocol_fee: U256,
        memo: Bytes,
        ) -> WithdrawCall  {
            self.shielder_account.prepare_call::<WithdrawCallType>(
            &self.params,
            &self.pk,
            self.token,
            amount,
            &WithdrawExtra {
                merkle_path: *merkle_path,
				to,
				relayer_address: relayer_fee_address,
				relayer_fee: total_cost_fee_token,
				contract_version: contract_version(),
				chain_id: U256::from(chain_id),
                mac_salt: Self::get_salt(),
				pocket_money,
				protocol_fee,
				memo,
			},
		)
    }


}