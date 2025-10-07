#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
ROOT_DIR="${SCRIPT_DIR}/.."

if [[ -n "${TESTNET:-}" ]]; then
  source "${SCRIPT_DIR}/testnet_env.sh"
else
  source "${SCRIPT_DIR}/local_env.sh"
fi
source "${SCRIPT_DIR}/utils.sh"

# ${1} - actor name
# ${2} - actor private key
scenario() {
  configure_cli ${1} ${2}
  log_progress "✅ CLI configured"

  relay_after_seconds=5
  withdrawal_amount=$(mtzero 50)

  #####################################################################################
  ################################# Native withdrawal #################################
  #####################################################################################
  relayer_fee_quote=$(quote_relayer_fee)
  max_relayer_fee=$(($relayer_fee_quote / 10 + $relayer_fee_quote)) # 110% of the quoted fee
  relay_after=$(date -d "+${relay_after_seconds} seconds" +%s)

  withdrawal_balance_before=$(cast balance -r "${NODE_RPC_URL}" "${WITHDRAWAL_PUBLIC_KEY}")

  ${1} schedule-withdraw $withdrawal_amount "${WITHDRAWAL_PUBLIC_KEY}" $relay_after $max_relayer_fee

  # wait until after relay time + scheduler interval + 2 seconds buffer
  sleep $(($relay_after_seconds + (${SCHEDULER_INTERVAL_SECS:-5} + 2)))

  withdrawal_balance_after=$(cast balance -r "${NODE_RPC_URL}" "${WITHDRAWAL_PUBLIC_KEY}")
  withdrawn=$((withdrawal_balance_after - withdrawal_balance_before))

  # Check if the withdrawn amount is within the expected range
  # (more than or equal to withdrawal_amount and less than or equal to withdrawal_amount + max_relayer_fee)
  if [ $withdrawn -lt $withdrawal_amount ] || [ $withdrawn -gt $(($withdrawal_amount + $max_relayer_fee)) ]; then
    log_progress "❌ Native withdrawal failed: expected between ${withdrawal_amount} and $(($withdrawal_amount + $max_relayer_fee)) increase, got ${withdrawn}"
    exit 1
  else
    log_progress "✅ Native withdrawal successful"
  fi

  # ####################################################################################
  # ################################# ERC20 withdrawal #################################
  # ####################################################################################
  pocket_money=$(mtzero 1)
  relayer_fee_quote=$(quote_relayer_fee ${ERC20_CONTRACT_ADDRESS_1} $pocket_money)
  max_relayer_fee=$(($relayer_fee_quote / 10 + $relayer_fee_quote)) # 110% of the quoted fee
  relay_after=$(date -d "+${relay_after_seconds} seconds" +%s)

  withdrawal_balance_before=$(cast balance -r "${NODE_RPC_URL}" "${WITHDRAWAL_PUBLIC_KEY}")
  withdrawal_erc20_balance_before=$(erc20_balance "${ERC20_CONTRACT_ADDRESS_1}" "${WITHDRAWAL_PUBLIC_KEY}")
  relayer_balance_before=$(erc20_balance "${ERC20_CONTRACT_ADDRESS_1}" "${FEE_DESTINATION}")

  ${1} schedule-withdraw-erc20 $withdrawal_amount "${WITHDRAWAL_PUBLIC_KEY}" "${ERC20_CONTRACT_ADDRESS_1}" $pocket_money $relay_after $max_relayer_fee

  # wait until after relay time + scheduler interval + 2 seconds buffer
  sleep $(($relay_after_seconds + (${SCHEDULER_INTERVAL_SECS:-5} + 2)))

  withdrawal_balance_after=$(cast balance -r "${NODE_RPC_URL}" "${WITHDRAWAL_PUBLIC_KEY}")
  withdrawal_erc20_balance_after=$(erc20_balance "${ERC20_CONTRACT_ADDRESS_1}" "${WITHDRAWAL_PUBLIC_KEY}")
  relayer_balance_after=$(erc20_balance "${ERC20_CONTRACT_ADDRESS_1}" "${FEE_DESTINATION}")

  withdrawn_erc20=$((withdrawal_erc20_balance_after - withdrawal_erc20_balance_before))
  if [ $withdrawn_erc20 -lt $withdrawal_amount ] || [ $withdrawn_erc20 -gt $(($withdrawal_amount + $max_relayer_fee)) ]; then
    log_progress "❌ ERC20 withdrawal failed: expected between ${withdrawal_amount} and $(($withdrawal_amount + $max_relayer_fee)) increase, got ${withdrawn_erc20}"
    exit 1
  else
    log_progress "✅ ERC20 withdrawal successful"
  fi

  pocket_money_sent=$((withdrawal_balance_after - withdrawal_balance_before))
  if [ $pocket_money_sent -ne $pocket_money ]; then
    log_progress "❌ ERC20 withdrawal failed: expected ${pocket_money} pocket money, got ${pocket_money_sent}"
    exit 1
  else
    log_progress "✅ ERC20 withdrawal pocket money successful"
  fi

  fee=$((relayer_balance_after - relayer_balance_before))
  if [ $fee -le 0 ]; then
    log_progress "❌ ERC20 withdrawal failed: expected ${withdrawal_amount} fee, got ${fee}"
    exit 1
  else
    log_progress "✅ ERC20 withdrawal fee successful"
  fi

  ${1} recover-scheduler-accounts native
  ${1} recover-scheduler-accounts ${ERC20_CONTRACT_ADDRESS_1}
  ${1} display-scheduler-accounts
}

run() {
  pushd $SCRIPT_DIR/.. &>> output.log

  setup

  start_scheduler

  scenario alice ${ALICE_PRIVATE_KEY}

  popd &>> output.log
}

trap cleanup EXIT SIGINT SIGTERM
rm -rf output.log
run
