import type { Hash, PublicClient, TransactionReceipt } from "viem";

const waitForTransactionInclusion = async ({
  hash,
  publicClient
}: {
  hash: Hash;
  publicClient: PublicClient;
}): Promise<TransactionReceipt> => {
  const receipt = await publicClient.waitForTransactionReceipt({ hash });

  // Since the introduction of flashblocks on Base chain,
  // awaiting the transaction receipt is not enough
  // to see the transaction effects.
  await new Promise<void>((resolve, reject) => {
    const unwatch = publicClient.watchBlockNumber({
      onBlockNumber: (blockNumber) => {
        if (blockNumber >= receipt.blockNumber) {
          unwatch();
          resolve();
        }
      },
      onError: (error) => {
        unwatch();
        reject(error);
      },
      emitOnBegin: true
    });
  });

  return receipt;
};

export default waitForTransactionInclusion;
