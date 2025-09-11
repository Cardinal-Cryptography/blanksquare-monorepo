/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_CRYPTO_CLIENT_TYPE: "wasm-full" | "wasm-light";
  readonly VITE_PROVER_SERVER_URL: string;
  readonly VITE_PCRS: string;
  readonly VITE_PUBLIC_THREADS: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
