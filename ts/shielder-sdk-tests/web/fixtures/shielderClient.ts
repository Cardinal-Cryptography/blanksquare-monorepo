import { mockedStorage } from "@/storage";
import type {
  InjectedStorageInterface,
  SendShielderTransaction,
  ShielderClient,
  Token
} from "@cardinal-cryptography/shielder-sdk";
import {
  createAccount,
  createPublicClient,
  type SeededAccount
} from "@tests/chainAccount";
import { type CallbacksFixture, setupCallbacks } from "./callbacks";

export interface ShielderClientFixture {
  shielderClient: ShielderClient;
  chainAccount: SeededAccount;
  storage: InjectedStorageInterface & {
    clear: () => void;
  };
  sendingTransaction: SendShielderTransaction;
  callbacks: CallbacksFixture;
  shield: (
    token: Token,
    amount: bigint
  ) => Promise<{
    tx: `0x${string}`;
    protocolFee: bigint;
  }>;
  withdraw: (
    token: Token,
    amount: bigint,
    to: `0x${string}`,
    pocketMoney: bigint
  ) => Promise<{
    tx: `0x${string}`;
    relayerFee: bigint;
    protocolFee: bigint;
  }>;
  withdrawManual: (
    token: Token,
    amount: bigint,
    to: `0x${string}`
  ) => Promise<{
    tx: `0x${string}`;
    protocolFee: bigint;
  }>;
  shieldedBalance: (token: Token) => Promise<bigint | null>;
}

export const setupShielderClient = async (
  chainConfig: {
    chainId: number;
    rpcHttpEndpoint: string;
    contractAddress: `0x${string}`;
    testnetPrivateKey: `0x${string}`;
  },
  relayerConfig: {
    url: string;
  },
  privateKey: `0x${string}`,
  shielderKey: `0x${string}`,
  referralEncryptionPublicKey: `0x${string}`,
  referralId: string
): Promise<ShielderClientFixture> => {
  const chainAccount: SeededAccount = createAccount(
    privateKey,
    chainConfig.chainId,
    chainConfig.rpcHttpEndpoint
  );
  const publicClient = createPublicClient(chainConfig.rpcHttpEndpoint);
  const storage = mockedStorage(shielderKey);
  const cryptoClient = await window.wasmCryptoClient.cryptoClient;
  const sendingTransaction: SendShielderTransaction = async (calldata) => {
    const tx = await chainAccount.sendTransaction({
      ...calldata,
      gas: 3000000n
    });
    return tx;
  };
  const callbacks = setupCallbacks();
  const shielderClient = window.shielder.createShielderClient({
    shielderSeedPrivateKey: shielderKey,
    chainId: BigInt(chainConfig.chainId),
    publicClient,
    contractAddress: chainConfig.contractAddress,
    relayerUrl: relayerConfig.url,
    storage,
    cryptoClient,
    callbacks: callbacks.callbacks,
    referral: {
      referralId,
      encryptionPublicKey: async () => {
        return Promise.resolve(referralEncryptionPublicKey);
      }
    }
  });

  return {
    shielderClient,
    chainAccount,
    storage,
    sendingTransaction,
    callbacks,
    shield: async (token, amount) => {
      const { amount: totalAmount, protocolFee } =
        await shielderClient.getProtocolShieldFee(amount, false);
      if (token.type === "erc20") {
        await chainAccount.approveERC20(
          token.address,
          chainConfig.contractAddress,
          totalAmount
        );
        // wait for 2 seconds to ensure all actions are processed
        await new Promise((resolve) => setTimeout(resolve, 2000));
      }
      return {
        tx: await shielderClient.shield(
          token,
          totalAmount,
          sendingTransaction,
          chainAccount.account.address,
          protocolFee
        ),
        protocolFee
      };
    },
    withdraw: async (token, amount, to, pocketMoney) => {
      const relayerFees = await shielderClient.getRelayerFees(
        token,
        pocketMoney
      );
      const relayerFee = relayerFees.fee_details.total_cost_fee_token;
      const { amount: totalAmount, protocolFee } =
        await shielderClient.getProtocolWithdrawFee(amount + relayerFee, false);
      return {
        tx: await shielderClient.withdraw(
          token,
          totalAmount,
          relayerFees,
          to,
          pocketMoney,
          protocolFee
        ),
        protocolFee,
        relayerFee
      };
    },
    withdrawManual: async (token, amount, to) => {
      const { amount: totalAmount, protocolFee } =
        await shielderClient.getProtocolWithdrawFee(amount, false);
      return {
        tx: await shielderClient.withdrawManual(
          token,
          totalAmount,
          to,
          sendingTransaction,
          chainAccount.account.address,
          protocolFee
        ),
        protocolFee
      };
    },
    shieldedBalance: async (token) => {
      return shielderClient
        .accountState(token)
        .then((state) => (state ? state.balance : null));
    }
  };
};
