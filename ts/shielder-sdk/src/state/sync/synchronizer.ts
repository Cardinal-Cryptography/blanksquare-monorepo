import { Mutex } from "async-mutex";
import { AccountRegistry } from "../accountRegistry";
import { TokenAccountFinder } from "./tokenAccountFinder";
import { Token } from "@/types";
import { ChainStateTransition } from "./chainStateTransition";
import {
  AccountState,
  AccountStateMerkleIndexed,
  ShielderTransaction
} from "../types";
import { firstAccountIndex } from "@/constants";
import { AccountOnchain } from "../accountOnchain";

export class StateSynchronizer {
  private singleTokenMutex: Mutex = new Mutex();

  constructor(
    private accountRegistry: AccountRegistry,
    private chainStateTransition: ChainStateTransition,
    private tokenAccountFinder: TokenAccountFinder,
    private accountOnchain: AccountOnchain,
    private syncCallback?: (shielderTransaction: ShielderTransaction) => unknown
  ) {}

  /**
   * Syncs the shielder state with the blockchain.
   * Emits the synced shielder transactions to the callback.
   * Locks to prevent concurrent storage changes.
   */
  async syncAllAccounts() {
    let accountIndex = firstAccountIndex;
    while (true) {
      let token =
        await this.accountRegistry.getTokenByAccountIndex(accountIndex);
      if (!token) {
        // try to find a token that has not been indexed yet
        token =
          await this.tokenAccountFinder.findTokenByAccountIndex(accountIndex);
      }
      if (!token) break; // no more tokens to sync
      await this.syncSingleAccount(token);
      accountIndex++;
    }
  }

  /**
   * Syncs the shielder state with the blockchain.
   * Emits the synced shielder transactions to the callback.
   * Locks to prevent concurrent storage changes.
   */
  async syncSingleAccount(token: Token) {
    await this.singleTokenMutex.runExclusive(async () => {
      const stateExisting = await this.accountRegistry.getAccountState(token);
      if (stateExisting) {
        await this.accountOnchain.validateAccountState(stateExisting);
      }
      let state: AccountState =
        stateExisting ??
        (await this.accountRegistry.createEmptyAccountState(token));
      while (true) {
        const stateTransition =
          await this.chainStateTransition.findStateTransition(state);
        if (!stateTransition) break;
        if (this.syncCallback) this.syncCallback(stateTransition.transaction);
        const newState: AccountStateMerkleIndexed = stateTransition.newState;
        state = newState;
        await this.accountRegistry.updateAccountState(token, newState);
      }
    });
  }
}
