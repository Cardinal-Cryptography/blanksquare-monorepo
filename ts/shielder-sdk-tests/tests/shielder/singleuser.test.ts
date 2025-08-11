import { erc20Token, nativeToken } from "@cardinal-cryptography/shielder-sdk";
import { expect } from "@playwright/test";
import { tokenContractAddresses } from "@tests/envConfig";
import { sdkTest } from "@tests/playwrightTestConfig";
import {
  clearStorageOp,
  recoverOp,
  shieldGasOp,
  shieldOp,
  withdrawManualOp,
  withdrawOp,
  type TestDescription
} from "@tests/types";

const ercToken = erc20Token(tokenContractAddresses[0] as `0x${string}`);

const ercTokenLowercase = erc20Token(
  ercToken.address.toLowerCase() as `0x${string}`
);

[
  {
    id: 1,
    actions: [
      // create native account, deposit, withdraw manually and via relayer
      {
        op: shieldOp(nativeToken(), 200n),
        actor: "alice"
      },
      {
        op: shieldGasOp(nativeToken()),
        actor: "alice"
      },
      {
        op: withdrawManualOp(nativeToken(), 50n, "bob"),
        actor: "alice"
      },
      {
        op: withdrawOp(nativeToken(), 70n, "bob", 0n),
        actor: "alice"
      },

      // create ERC20 account, deposit, withdraw manually and via relayer
      { op: shieldOp(ercToken, 100n), actor: "alice" },
      {
        op: shieldGasOp(ercTokenLowercase),
        actor: "alice"
      },
      {
        op: withdrawManualOp(ercToken, 50n, "bob"),
        actor: "alice"
      },
      {
        op: withdrawOp(ercToken, 70n, "bob", 0n),
        actor: "alice"
      },

      // clear and recover
      { op: clearStorageOp(), actor: "alice" },
      { op: recoverOp(), actor: "alice" },

      // shield again
      {
        op: shieldOp(nativeToken(), 100n),
        actor: "alice"
      },
      {
        op: shieldGasOp(nativeToken()),
        actor: "alice"
      },
      { op: shieldOp(ercToken, 100n), actor: "alice" },
      {
        op: shieldGasOp(ercTokenLowercase),
        actor: "alice"
      },

      // withdraw again via relayer
      {
        op: withdrawOp(nativeToken(), 100n, "charlie", 0n),
        actor: "alice"
      },
      {
        op: withdrawOp(ercToken, 100n, "charlie", 0n),
        actor: "alice"
      }
    ]
  } as TestDescription
].forEach(({ id, actions }: TestDescription) => {
  sdkTest(
    `shield, withdraw, recover, validate balances&history. test no ${id}`,
    async ({ workerPage, perTestConfig }) => {
      const isGood = await workerPage.evaluate(
        async ({ perTestConfig: { testFixture }, actions }) => {
          for (const action of actions) {
            await testFixture.executeAction(action);

            await testFixture.validateWithdrawnBalance(action.actor);

            if (action.op.type !== "clearStorage") {
              await testFixture.validateShielderBalance(action.actor);
              await testFixture.validateShielderHistory(action.actor);
            }
          }
          return true;
        },
        {
          perTestConfig,
          actions
        }
      );
      expect(isGood).toBe(true);
    }
  );
});
