import { beforeEach, describe, expect, it, vitest } from "vitest";
import {
  Scalar,
  scalarsEqual,
  scalarToBigint,
  WithdrawAdvice,
  WithdrawPubInputs
} from "@cardinal-cryptography/shielder-sdk-crypto";

import { MockedCryptoClient, hashedNote } from "../helpers";

import { WithdrawAction } from "../../src/actions/withdraw";
import { IContract } from "../../src/chain/contract";
import {
  IRelayer,
  quotedFeesFromExpectedTokenFee,
  WithdrawResponse
} from "../../src/chain/relayer";
import { nativeToken } from "../../src/utils";
import { OutdatedSdkError } from "../../src/errors";
import { AccountStateMerkleIndexed } from "../../src/state/types";
import { Token } from "../../src/types";

describe("WithdrawAction", () => {
  let cryptoClient: MockedCryptoClient;
  let contract: IContract;
  let relayer: IRelayer;
  let action: WithdrawAction;
  let state: AccountStateMerkleIndexed;
  const stateNonce = 1n;
  const prevNullifier = Scalar.fromBigint(2n);
  const mockAddress =
    "0x1234567890123456789012345678901234567890" as `0x${string}`;
  const mockRelayerAddress =
    "0x0987654321098765432109876543210987654321" as `0x${string}`;

  const mockedPath = [0n, 1n];
  let mockedMerkleRoot: Scalar;

  const mockProtocolFee = 0n;
  const mockMemo = new Uint8Array();

  beforeEach(async () => {
    cryptoClient = new MockedCryptoClient();
    mockedMerkleRoot = await cryptoClient.hasher.poseidonHash([
      Scalar.fromBigint(0n),
      Scalar.fromBigint(1n)
    ]);
    contract = {
      getAddress: vitest.fn().mockReturnValue(mockAddress),
      depositCalldata: vitest
        .fn<
          (
            expectedContractVersion: `0x${string}`,
            from: `0x${string}`,
            oldNoteNullifierHash: bigint,
            newNote: bigint,
            merkleRoot: bigint,
            amount: bigint,
            proof: Uint8Array
          ) => Promise<`0x${string}`>
        >()
        .mockResolvedValue("0xmockedCalldata"),
      getMerklePath: vitest
        .fn<(idx: bigint) => Promise<readonly bigint[]>>()
        .mockResolvedValue([...mockedPath, scalarToBigint(mockedMerkleRoot)])
    } as unknown as IContract;
    relayer = {
      address: vitest
        .fn<() => Promise<`0x${string}`>>()
        .mockResolvedValue(mockRelayerAddress),
      withdraw: vitest
        .fn<
          (
            expectedContractVersion: `0x${string}`,
            oldNullifierHash: bigint,
            newNote: bigint,
            merkleRoot: bigint,
            amount: bigint,
            proof: Uint8Array,
            withdrawAddress: `0x${string}`
          ) => Promise<WithdrawResponse>
        >()
        .mockResolvedValue({
          tx_hash: "0xtxHash" as `0x${string}`
        })
    } as unknown as IRelayer;
    action = new WithdrawAction(contract, relayer, cryptoClient, 1n);

    const id = Scalar.fromBigint(123n);
    state = {
      id,
      nonce: stateNonce,
      balance: 5n,
      currentNote: await hashedNote(id, prevNullifier, Scalar.fromBigint(5n)),
      currentNoteIndex: 100n,
      token: nativeToken()
    };
  });

  describe("rawWithdraw", () => {
    it("should transform an existing state", async () => {
      const amount = 2n;
      const expectedAmount = state.balance - amount;
      const result = await action.rawWithdraw(state, amount);

      expect(result).not.toBeNull();
      if (!result) {
        throw new Error("result is null");
      }
      expect(result.balance).toBe(expectedAmount);
      expect(result.nonce).toBe(2n);
      // Nullifier should be secret manager's output
      const { nullifier: newNullifier } =
        await cryptoClient.secretManager.getSecrets(
          state.id,
          Number(state.nonce)
        );
      // Note should be hash of [version, id, nullifier, amount]
      const expectedNote = await hashedNote(
        state.id,
        newNullifier,
        Scalar.fromBigint(expectedAmount)
      );
      expect(scalarsEqual(result.currentNote, expectedNote)).toBe(true);
    });
  });

  describe("generateCalldata", () => {
    it("should generate valid calldata", async () => {
      const amount = 2n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const address =
        "0x1234567890123456789012345678901234567890" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 0n;
      const calldata = await action.generateCalldata(
        state,
        amount,
        mockRelayerAddress,
        quotedFeesFromExpectedTokenFee(totalFee),
        address,
        expectedVersion,
        pocketMoney,
        mockProtocolFee,
        mockMemo
      );

      // Verify the proof
      const isValid = await cryptoClient.withdrawCircuit.verify(
        calldata.calldata.proof,
        calldata.calldata.pubInputs
      );
      expect(isValid).toBe(true);

      // Amount should be equal to input amount
      expect(calldata.amount).toBe(amount);

      // Expected contract version should be equal to input expected version
      expect(calldata.expectedContractVersion).toBe(expectedVersion);
    });

    it("should throw on balance less than amount", async () => {
      const amount = 6n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 0n;
      await expect(
        action.generateCalldata(
          state,
          amount,
          mockRelayerAddress,
          quotedFeesFromExpectedTokenFee(totalFee),
          mockAddress,
          expectedVersion,
          pocketMoney,
        mockProtocolFee,
        mockMemo
        )
      ).rejects.toThrow("Insufficient funds");
    });

    it("should throw on non-zero pocket money for native withdrawal", async () => {
      const amount = 2n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 1n;
      await expect(
        action.generateCalldata(
          state,
          amount,
          mockRelayerAddress,
          quotedFeesFromExpectedTokenFee(totalFee),
          mockAddress,
          expectedVersion,
          pocketMoney,
        mockProtocolFee,
        mockMemo
        )
      ).rejects.toThrow("Pocket money is not supported for native withdrawal");
    });

    it("should throw on amount less than fee", async () => {
      const amount = 1n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 2n;
      const pocketMoney = 0n;
      await expect(
        action.generateCalldata(
          state,
          amount,
          mockRelayerAddress,
          quotedFeesFromExpectedTokenFee(totalFee),
          mockAddress,
          expectedVersion,
          pocketMoney,
        mockProtocolFee,
        mockMemo
        )
      ).rejects.toThrow(`Amount must be greater than the sum of fees: Relayer Fee: ${totalFee}, Protocol Fee: ${mockProtocolFee}`);
    });

    it("should throw on incorrect prover inputs", async () => {
      const amount = 2n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 0n;

      cryptoClient.withdrawCircuit.prove = vitest
        .fn<
          (values: WithdrawAdvice<Scalar>) => Promise<{
            proof: Uint8Array;
            pubInputs: WithdrawPubInputs<Scalar>;
          }>
        >()
        .mockRejectedValue("error");

      await expect(
        action.generateCalldata(
          state,
          amount,
          mockRelayerAddress,
          quotedFeesFromExpectedTokenFee(totalFee),
          mockAddress,
          expectedVersion,
          pocketMoney,
        mockProtocolFee,
        mockMemo
        )
      ).rejects.toThrow("Failed to prove withdrawal:");
    });

    it("should throw on failed verification", async () => {
      const amount = 2n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 0n;

      cryptoClient.withdrawCircuit.verify = vitest
        .fn<
          (
            proof: Uint8Array,
            values: WithdrawPubInputs<Scalar>
          ) => Promise<boolean>
        >()
        .mockResolvedValue(false);

      await expect(
        action.generateCalldata(
          state,
          amount,
          mockRelayerAddress,
          quotedFeesFromExpectedTokenFee(totalFee),
          mockAddress,
          expectedVersion,
          pocketMoney,
        mockProtocolFee,
        mockMemo
        )
      ).rejects.toThrow("Withdrawal proof verification failed");
    });
  });

  describe("sendCalldataWithRelayer", () => {
    it("should send transaction with correct parameters", async () => {
      const amount = 2n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 0n;
      const calldata = await action.generateCalldata(
        state,
        amount,
        mockRelayerAddress,
        quotedFeesFromExpectedTokenFee(totalFee),
        mockAddress,
        expectedVersion,
        pocketMoney,
        mockProtocolFee,
        mockMemo
      );

      const txHash = await action.sendCalldataWithRelayer(calldata);

      expect(relayer.withdraw).toHaveBeenCalledWith(
        expectedVersion,
        nativeToken(),
        scalarToBigint(calldata.calldata.pubInputs.hNullifierOld),
        scalarToBigint(calldata.calldata.pubInputs.hNoteNew),
        scalarToBigint(calldata.calldata.pubInputs.merkleRoot),
        calldata.amount,
        calldata.calldata.proof,
        mockAddress,
        scalarToBigint(calldata.calldata.pubInputs.macSalt),
        scalarToBigint(calldata.calldata.pubInputs.macCommitment),
        calldata.pocketMoney,
        quotedFeesFromExpectedTokenFee(totalFee),
        calldata.memo
      );

      expect(txHash).toBe("0xtxHash");
    });

    it("should throw on rejected contract version", async () => {
      const amount = 2n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 0n;
      const calldata = await action.generateCalldata(
        state,
        amount,
        mockRelayerAddress,
        quotedFeesFromExpectedTokenFee(totalFee),
        mockAddress,
        expectedVersion,
        pocketMoney,
        mockProtocolFee,
        mockMemo
      );

      const mockedErr = new OutdatedSdkError("123");

      relayer.withdraw = vitest
        .fn<
          (
            expectedContractVersion: `0x${string}`,
            token: Token,
            oldNullifierHash: bigint,
            newNote: bigint,
            merkleRoot: bigint,
            amount: bigint,
            proof: Uint8Array,
            withdrawalAddress: `0x${string}`,
            macSalt: bigint,
            macCommitment: bigint
          ) => Promise<WithdrawResponse>
        >()
        .mockRejectedValue(mockedErr);

      await expect(
        action.sendCalldataWithRelayer(calldata)
      ).rejects.toThrowError(mockedErr);
    });

    it("should throw on other errors during send", async () => {
      const amount = 2n;
      const expectedVersion = "0xversio" as `0x${string}`;
      const totalFee = 1n;
      const pocketMoney = 0n;
      const calldata = await action.generateCalldata(
        state,
        amount,
        mockRelayerAddress,
        quotedFeesFromExpectedTokenFee(totalFee),
        mockAddress,
        expectedVersion,
        pocketMoney,
        mockProtocolFee,
        mockMemo
      );

      relayer.withdraw = vitest
        .fn<
          (
            expectedContractVersion: `0x${string}`,
            token: Token,
            oldNullifierHash: bigint,
            newNote: bigint,
            merkleRoot: bigint,
            amount: bigint,
            proof: Uint8Array,
            withdrawalAddress: `0x${string}`,
            macSalt: bigint,
            macCommitment: bigint
          ) => Promise<WithdrawResponse>
        >()
        .mockRejectedValue(new Error("mocked contract rejection"));

      await expect(action.sendCalldataWithRelayer(calldata)).rejects.toThrow(
        "Failed to withdraw: Error: mocked contract rejection"
      );
    });
  });
});
