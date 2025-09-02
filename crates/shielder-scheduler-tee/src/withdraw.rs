use std::array;
use rand::rngs::OsRng;

use alloy_primitives::{Address, Bytes, U256};
use shielder_account::call_data::{WithdrawCall, WithdrawExtra};
use shielder_account::{Token};
use shielder_circuits::circuits::{Params, ProvingKey};
use shielder_circuits::{generate_proof, Field, Fr, ProverKnowledge, PublicInputProvider};
use shielder_circuits::marshall::{self, unmarshall_params};
use shielder_circuits::withdraw::{WithdrawInstance, WithdrawProverKnowledge};

use shielder_contract::WithdrawCommitment;
use shielder_relayer::RelayCalldata;
use shielder_setup::consts::{ARITY, TREE_HEIGHT};
use shielder_setup::version::contract_version;
use type_conversions::{address_to_field, field_to_address, field_to_u256, u256_to_field};


#[derive(Clone, Debug)]
pub struct WithdrawCircuit {
    params: Params,
    pk: ProvingKey,
    token: Token,
    shielder_account_id: U256
}

impl WithdrawCircuit {
    pub fn new(shielder_account_id: U256, token: Token) -> Self {
        WithdrawCircuit {
            params: Self::get_params(
                include_bytes!("../artifacts/withdraw/params.bin"),
            ),
            pk: Self::get_pk(include_bytes!("../artifacts/withdraw/pk.bin")),
            shielder_account_id,
            token
        }
    }

    fn get_params(params_buf: &[u8]) -> Params {
        unmarshall_params(params_buf)
            .expect("Failed to unmarshall params")
    }

    fn get_pk(pk_buf: &[u8]) -> ProvingKey {
        let (_, pk) = marshall::unmarshall_pk::<
            <WithdrawProverKnowledge<shielder_circuits::Fr> as shielder_circuits::ProverKnowledge>::Circuit,
        >(pk_buf)
        .expect("Failed to unmarshall pk");

        pk
    }

    fn prepare_prover_knowledge(
        &self,
        token_address: Address,
        amount: U256,
        extra: &WithdrawExtra,
        nullifier_old: U256,
        nullifier_new: U256,
        account_old_balance: U256,
    ) -> WithdrawProverKnowledge<shielder_circuits::Fr> {
        let commitment = WithdrawCommitment {
            contract_version: extra.contract_version,
            withdraw_address: extra.to,
            relayer_address: extra.relayer_address,
            relayer_fee: extra.relayer_fee,
            chain_id: extra.chain_id,
            pocket_money: extra.pocket_money,
            protocol_fee: extra.protocol_fee,
            memo: extra.memo.clone(),
        }
        .commitment_hash();

        WithdrawProverKnowledge {
            id: u256_to_field(self.shielder_account_id),
            nullifier_old: u256_to_field(nullifier_old),
            account_old_balance: u256_to_field(account_old_balance),
            token_address: address_to_field(token_address),
            path: Self::map_path_to_field(extra.merkle_path),
            withdrawal_value: u256_to_field(amount),
            nullifier_new: u256_to_field(nullifier_new),
            commitment: u256_to_field(commitment),
            mac_salt: u256_to_field(extra.mac_salt),
        }
    }

    fn map_path_to_field(path: [[U256; ARITY]; TREE_HEIGHT]) -> [[Fr; ARITY]; TREE_HEIGHT] {
        let mut result = [[Fr::ZERO; ARITY]; TREE_HEIGHT];
        for (i, row) in path.iter().enumerate() {
            for (j, element) in row.iter().enumerate() {
                result[i][j] = u256_to_field(*element);
            }
        }
        result
}


    fn get_salt() -> U256 {
	    U256::from_limbs(array::from_fn(|_| rand::random()))
    }

    pub fn get_relayer_calldata(&self, amount: U256, 
        to: Address, 
        merkle_path: [[U256; ARITY]; TREE_HEIGHT], 
        chain_id: U256, 
        total_cost_fee_token: U256, 
        pocket_money: U256, 
        relayer_fee_address: Address,
        protocol_fee: U256,
        memo: Bytes,
        nullifier_old: U256,
        nullifier_new: U256,
        account_old_balance: U256,
        merkle_root: U256,
    ) -> RelayCalldata  {
        let extra = WithdrawExtra {
                merkle_path,
				to,
				relayer_address: relayer_fee_address,
				relayer_fee: total_cost_fee_token,
				contract_version: contract_version(),
				chain_id,
                mac_salt: Self::get_salt(),
				pocket_money,
				protocol_fee,
				memo,
			};

        let prover_knowledge = self.prepare_prover_knowledge(
            self.token.address(),
            amount,
            &extra,
            nullifier_old,
            nullifier_new,
            account_old_balance,
        );

        let proof = generate_proof(
            &self.params,
            &self.pk,
            prover_knowledge.create_circuit(),
            &prover_knowledge.serialize_public_input(),
            &mut OsRng,
        );

        let withdraw_call = WithdrawCall {
            expected_contract_version: contract_version().to_bytes(),
            token: field_to_address(prover_knowledge.token_address).into(),
            amount: field_to_u256(prover_knowledge.compute_public_input(WithdrawInstance::WithdrawalValue)),
            withdrawal_address: extra.to,
            merkle_root: field_to_u256(prover_knowledge.compute_public_input(WithdrawInstance::MerkleRoot)),
            old_nullifier_hash: field_to_u256(prover_knowledge.compute_public_input(WithdrawInstance::HashedOldNullifier)),
            new_note: field_to_u256(prover_knowledge.compute_public_input(WithdrawInstance::HashedNewNote)),
            proof: Bytes::from(proof),
            relayer_address: extra.relayer_address,
            relayer_fee: extra.relayer_fee,
            mac_salt: field_to_u256(prover_knowledge.compute_public_input(WithdrawInstance::MacSalt)),
            mac_commitment: field_to_u256(prover_knowledge.compute_public_input(WithdrawInstance::MacCommitment)),
            pocket_money: extra.pocket_money,
            memo: extra.memo.clone(),
        };


        RelayCalldata {
            expected_contract_version: contract_version().to_bytes(),
            amount,
            withdraw_address: to,
            merkle_root,
            nullifier_hash: withdraw_call.old_nullifier_hash,
            new_note: withdraw_call.new_note,
            proof: withdraw_call.proof,
            fee_token: Token::Native,
            fee_amount: withdraw_call.relayer_fee,
            mac_salt: withdraw_call.mac_salt,
            mac_commitment: withdraw_call.mac_commitment,
            pocket_money,
            memo: Bytes::from(vec![]),
        }
    }


}