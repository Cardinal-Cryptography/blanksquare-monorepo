import { getChainConfig, getRelayerConfig } from "@tests/envConfig";
import {
  ACCOUNT_NAMES,
  INITIAL_EVM_BALANCE_GAS_POINTS
} from "@tests/constants";
import type { AccountValue } from "@tests/types";
import { generatePrivateKey, privateKeyToAddress } from "viem/accounts";
import { createAccount } from "@tests/chainAccount";
import { createPublicClient, http } from "viem";

export type GlobalConfigFixture = {
  chainConfig: ReturnType<typeof getChainConfig>;
  relayerConfig: ReturnType<typeof getRelayerConfig>;
  privateKeys: AccountValue<`0x${string}`>;
};

export const globalConfigFixture = async (
  // eslint-disable-next-line no-empty-pattern
  {},
  use: (r: GlobalConfigFixture) => Promise<void>
) => {
  const chainConfig = getChainConfig();
  const relayerConfig = getRelayerConfig();

  const privateKeys = {} as AccountValue<`0x${string}`>;

  for (const name of ACCOUNT_NAMES) {
    privateKeys[name] = generatePrivateKey();
  }

  const faucet = createAccount(
    chainConfig.testnetPrivateKey,
    chainConfig.chainId,
    chainConfig.rpcHttpEndpoint
  );

  const publicClient = createPublicClient({
    transport: http(chainConfig.rpcHttpEndpoint)
  });

  const gasPrice = await publicClient.getGasPrice();

  // initial balance is gas points constant, multiplied by gas price and adjusted with +20%
  const initialEvmBalance =
    (INITIAL_EVM_BALANCE_GAS_POINTS * gasPrice * 120n) / 100n;

  for (const name of ACCOUNT_NAMES) {
    await faucet.sendNative(
      privateKeyToAddress(privateKeys[name]),
      initialEvmBalance
    );
    for (const tokenAddress of chainConfig.tokenContractAddresses) {
      await faucet.sendERC20(
        tokenAddress as `0x${string}`,
        privateKeyToAddress(privateKeys[name]),
        initialEvmBalance
      );
    }
  }

  await use({ chainConfig, relayerConfig, privateKeys });
};
