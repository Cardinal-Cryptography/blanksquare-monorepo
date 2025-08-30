import { Token } from "@/types";
import { Scalar } from "@cardinal-cryptography/shielder-sdk-crypto";
import { Address, Hex } from "viem";

export type AccountState = {
  /**
   * Account id, a scalar derived from the private key.
   */
  id: Scalar;
  /**
   * Account token
   */
  token: Token;
  /**
   * Account nonce, increments for each new action.
   */
  nonce: bigint;
  /**
   * Account balance, in wei.
   */
  balance: bigint;
  /**
   * Hash of the last note.
   */
  currentNote: Scalar;
};

export type AccountStateMerkleIndexed = AccountState & {
  /**
   * Merkle tree index of the last note.
   */
  currentNoteIndex: bigint;
};

type ShielderTransactionBase = {
  amount: bigint;
  txHash: Hex;
  block: bigint;
  token: Token;
  newNote: Scalar;
  protocolFee: bigint;
  memo: `0x${string}`;
};

export type ShielderTransaction = ShielderTransactionBase &
  (
    | {
        type: "NewAccount" | "Deposit";
        to?: undefined;
        relayerFee?: undefined;
        pocketMoney?: undefined;
      }
    | {
        type: "Withdraw";
        to: Address;
        relayerFee: bigint;
        pocketMoney: bigint;
      }
  );
