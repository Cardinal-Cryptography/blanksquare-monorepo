import { verifyAttestation } from "@/nitro-attestation";
import {
  decrypt,
  encrypt,
  generateKeypair
} from "@cardinal-cryptography/ecies-encryption-lib";
import {
  base64ToBytes,
  bytesToBase64,
  bytesToObject,
  objectToBytes
} from "@/utils";

type TeePublicKeyResponse = {
  TeePublicKey: {
    // secp256k1 public key in hex format
    public_key: string;
    // base64-encoded AWS Nitro attestation document
    attestation_document: string;
  };
};

export class TeeClient {
  provingServiceUrl: string | undefined;

  provingServicePublicKey: string | undefined;

  async init(
    provingServiceUrl: string,
    withoutAttestation: boolean
  ): Promise<void> {
    this.provingServiceUrl = provingServiceUrl;

    const response = await fetch(`${this.provingServiceUrl}/public_key`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json"
      }
    }).catch((e) => {
      throw new Error(`Failed to fetch public key from TEE service: ${e}`);
    });
    if (!response.ok) {
      throw new Error(
        `Failed to fetch public key from TEE service: ${response.statusText}`
      );
    }
    const data: TeePublicKeyResponse =
      (await response.json()) as TeePublicKeyResponse;
    if (!data.TeePublicKey) {
      throw new Error(
        "Invalid response from TEE service: missing TeePublicKey field"
      );
    }
    if (!data.TeePublicKey.public_key) {
      throw new Error("Invalid response from TEE service: missing public key");
    }

    this.provingServicePublicKey = data.TeePublicKey.public_key;

    if (withoutAttestation) return;

    if (!data.TeePublicKey.attestation_document) {
      throw new Error(
        "Invalid response from TEE service: missing attestation document"
      );
    }

    await verifyAttestation(data.TeePublicKey.attestation_document);
  }

  async prove(
    circuitType: "NewAccount" | "Deposit" | "Withdraw",
    witness: Uint8Array
  ): Promise<{
    proof: Uint8Array;
    pubInputs: Uint8Array;
  }> {
    if (!this.provingServiceUrl || !this.provingServicePublicKey) {
      throw new Error(
        "TeeClient is not initialized. Call init() before proving."
      );
    }

    const { sk, pk } = generateKeypair();

    // Payload must be a JSON object with the following fields:
    // - circuit_type: string, one of "NewAccount", "Deposit", "Withdraw"
    // - circuit_inputs: base64-encoded Uint8Array
    // - user_public_key: base64-encoded secp256k1 public key
    const payload = objectToBytes({
      circuit_type: circuitType,
      circuit_inputs: bytesToBase64(witness),
      user_public_key: bytesToBase64(pk)
    });

    // Encrypt the payload with the TEE service public key
    const encryptedPayload = await encrypt(
      payload,
      this.provingServicePublicKey
    ).catch((e) => {
      throw new Error(`Failed to encrypt payload: ${e}`);
    });

    // Then encode it to base64 for transmission
    const response = await fetch(`${this.provingServiceUrl}/proof`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        payload: bytesToBase64(encryptedPayload)
      })
    });

    if (!response.ok) {
      throw new Error(
        `Failed to prove with TEE service: ${JSON.stringify(response)}`
      );
    }
    const data = (await response.json()) as {
      EncryptedProof: {
        payload: string;
      };
    };

    if (!data.EncryptedProof) {
      throw new Error(
        "Invalid response from TEE service: missing EncryptedProof"
      );
    }
    if (!data.EncryptedProof.payload) {
      throw new Error("Invalid response from TEE service: missing payload");
    }

    // Decrypt the payload with the user private key
    // We expect the payload to be base64-encoded
    const decryptedPayload = await decrypt(
      base64ToBytes(data.EncryptedProof.payload),
      sk
    ).catch((e) => {
      throw new Error(`Failed to decrypt payload: ${e}`);
    });

    // The decrypted payload should be a JSON object with the following fields:
    // - proof: base64-encoded Uint8Array
    // - pub_inputs: base64-encoded Uint8Array
    const decryptedData = bytesToObject(decryptedPayload) as {
      proof: string;
      pub_inputs: string;
    };

    return {
      proof: base64ToBytes(decryptedData.proof),
      pubInputs: base64ToBytes(decryptedData.pub_inputs)
    };
  }
}
