export const cryptoClientType: "wasm-full" | "wasm-light" =
  import.meta.env.VITE_CRYPTO_CLIENT_TYPE || "wasm-full";
export const proverServerUrl = import.meta.env.VITE_PROVER_SERVER_URL || "";
export const pcrs = import.meta.env.VITE_PCRS
  ? new Map<string, string>(
      Object.entries(
        JSON.parse(import.meta.env.VITE_PCRS) as Record<string, string>
      )
    )
  : undefined;
