use std::{env, fs::File, io::Write};

use alloy_primitives::U256;
use integration_tests::{
    calls::{
        deposit::{invoke_call as deposit_call, prepare_call as deposit_calldata},
        new_account::{invoke_call as new_account_call, prepare_call as new_account_calldata},
        withdraw::{invoke_call as withdraw_call, prepare_args, prepare_call as withdraw_calldata},
    },
    deploy::{
        deployment, Deployment, MEMO_BYTES, PROTOCOL_FEES, ZERO_MEMO_BYTES, ZERO_PROTOCOL_FEES,
    },
    deposit_proving_params, new_account_proving_params, withdraw_proving_params, TestToken,
};
use shielder_account::{
    call_data::{DepositCall, NewAccountCall, WithdrawCall},
    ShielderAccount,
};

#[derive(Debug)]
enum Calldata {
    NewAccount(NewAccountCall),
    Deposit(DepositCall),
    Withdraw(WithdrawCall),
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let mut file = File::create(filename).unwrap();

    let mut deployment = deployment(
        &new_account_proving_params(),
        &deposit_proving_params(),
        &withdraw_proving_params(),
    );

    let mut shielder_account =
        ShielderAccount::new(U256::MAX, TestToken::Native.token(&deployment));
    let calldata = new_account_calldata(
        &mut deployment,
        &mut shielder_account,
        TestToken::Native,
        U256::from(1),
        ZERO_MEMO_BYTES,
    );
    let _ = new_account_call(&mut deployment, &mut shielder_account, &calldata);

    let mut content: Vec<u8> = vec![];

    for (account_id, (token, protocol_fees_bps, memo)) in [
        (TestToken::Native, ZERO_PROTOCOL_FEES, ZERO_MEMO_BYTES),
        (TestToken::ERC20, ZERO_PROTOCOL_FEES, ZERO_MEMO_BYTES),
        (TestToken::Native, PROTOCOL_FEES, ZERO_MEMO_BYTES),
        (TestToken::ERC20, PROTOCOL_FEES, ZERO_MEMO_BYTES),
        (TestToken::Native, ZERO_PROTOCOL_FEES, MEMO_BYTES),
        (TestToken::ERC20, ZERO_PROTOCOL_FEES, MEMO_BYTES),
        (TestToken::Native, PROTOCOL_FEES, MEMO_BYTES),
        (TestToken::ERC20, PROTOCOL_FEES, MEMO_BYTES),
    ]
    .into_iter()
    .enumerate()
    {
        let label = format!(
            "{}{}{}",
            token,
            if protocol_fees_bps.protocol_deposit_fee_bps != U256::ZERO
                && protocol_fees_bps.protocol_withdraw_fee_bps != U256::ZERO
            {
                "Fees"
            } else {
                ""
            },
            if !memo.is_empty() { "Memo" } else { "" }
        );
        deployment.set_protocol_fees(&protocol_fees_bps);
        let mut shielder_account =
            ShielderAccount::new(U256::from(account_id.pow(2)), token.token(&deployment));
        let amount = U256::from(10);
        let calldata = Calldata::NewAccount(new_account_calldata(
            &mut deployment,
            &mut shielder_account,
            token,
            amount,
            memo.clone(),
        ));

        measure_gas(
            &format!("NewAccount{}", label),
            &calldata,
            &mut deployment,
            &mut shielder_account,
            &mut content,
        );

        let calldata = Calldata::Deposit(
            deposit_calldata(
                &mut deployment,
                &mut shielder_account,
                token,
                amount,
                memo.clone(),
            )
            .0,
        );

        measure_gas(
            &format!("Deposit{}", label),
            &calldata,
            &mut deployment,
            &mut shielder_account,
            &mut content,
        );

        let pocket_money = match token {
            TestToken::Native => U256::from(0),
            TestToken::ERC20 => U256::from(1),
        };
        let calldata = Calldata::Withdraw(
            withdraw_calldata(
                &mut deployment,
                &mut shielder_account,
                prepare_args(token, amount, U256::from(1), pocket_money, memo.clone()),
            )
            .0,
        );

        measure_gas(
            &format!("Withdraw{}", label),
            &calldata,
            &mut deployment,
            &mut shielder_account,
            &mut content,
        );
    }

    file.write_all(&content).unwrap();
}

fn measure_gas(
    label: &str,
    calldata: &Calldata,
    deployment: &mut Deployment,
    shielder_account: &mut ShielderAccount,
    content: &mut Vec<u8>,
) {
    let gas_used = match calldata {
        Calldata::NewAccount(calldata) => {
            new_account_call(deployment, shielder_account, calldata)
                .unwrap()
                .1
                .gas_used
        }
        Calldata::Deposit(calldata) => {
            deposit_call(deployment, shielder_account, calldata)
                .unwrap()
                .1
                .gas_used
        }
        Calldata::Withdraw(calldata) => {
            withdraw_call(deployment, shielder_account, calldata)
                .unwrap()
                .1
                .gas_used
        }
    };

    content.extend(&mut format!("{label}: {gas_used}\n").as_bytes().iter());
}
