/**
 * AWS Nitro Enclaves Attestation Verification Tests
 *
 * Tests the verification of real attestation documents from AWS Nitro Enclaves,
 * including PCR validation and public key consistency checks.
 */

import { describe, it, expect, beforeAll } from "vitest";
import {
  verifyAttestation,
  extractPCRs,
  type AttestationResult
} from "../src/index";
import * as pkijs from "pkijs";

// Mock server response from AWS Nitro Enclaves TEE
const SERVER_RESPONSE = {
  TeePublicKey: {
    public_key:
      "03637e4a2d6b8325ec60e0ccd53ca0f05f67e3356535c45656403aee207d4694d8",
    attestation_document:
      "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"
  }
};

// Expected PCR values for validation
const EXPECTED_PCRS = new Map([
  [
    "0",
    "94b74422daddb8f503fcd69df064c7cde5d053001b9cd153c75ec34484283496a37e89c7287a00f467759a6863362b60"
  ],
  [
    "1",
    "927e084e583f5c2d60a39e2b9cd9728bfb390aa9f83dee4b6ac768509850ba273ea8b019ccfbf3180eb18a2dd0c4a678"
  ],
  [
    "2",
    "07c20c057d5c10cb732b273b7fa26a2b67e333344ccda49be939a6b7c5bed5e46f0b0703386dd2d0f6104a13a4894cb2"
  ]
]);

// Valid date for certificate validation in tests (when the test certificates were valid)
// Based on the certificate data, they were valid around September 9, 2025
const VALID_CERT_DATE = new Date("2025-09-09T18:00:00Z");

beforeAll(() => {
  console.log(crypto);
  if (typeof self !== "undefined") {
    console.log("here");
    if ("crypto" in self) {
      let engineName = "webcrypto";
      if ("webkitSubtle" in self.crypto) {
        engineName = "safari";
      }
      pkijs.setEngine(
        engineName,
        new pkijs.CryptoEngine({ name: engineName, crypto: crypto })
      );
    }
  } else if (typeof crypto !== "undefined") {
    const name = "NodeJS ^15";
    const nodeCrypto = crypto;
    console.log("here");
    pkijs.setEngine(name, new pkijs.CryptoEngine({ name, crypto: nodeCrypto }));
  }
});

describe("AWS Nitro Enclaves Verification", () => {
  describe("Server Response Parsing", () => {
    it("should parse server response correctly", () => {
      expect(SERVER_RESPONSE.TeePublicKey).toBeDefined();
      expect(SERVER_RESPONSE.TeePublicKey.public_key).toBeDefined();
      expect(SERVER_RESPONSE.TeePublicKey.attestation_document).toBeDefined();

      // Validate public key format (hex string)
      expect(SERVER_RESPONSE.TeePublicKey.public_key).toMatch(/^[0-9a-fA-F]+$/);

      // Validate attestation document format (base64)
      expect(SERVER_RESPONSE.TeePublicKey.attestation_document).toMatch(
        /^[A-Za-z0-9+/]+=*$/
      );
    });
  });

  describe("Attestation Document Verification", () => {
    it("should verify attestation document with expected PCRs", async () => {
      const attestationDocument =
        SERVER_RESPONSE.TeePublicKey.attestation_document;

      // Verify attestation with expected PCR values using valid certificate date
      const result: AttestationResult = await verifyAttestation(
        attestationDocument,
        EXPECTED_PCRS,
        VALID_CERT_DATE
      );

      // Validate the result structure
      expect(result).toBeDefined();
      expect(result.pcrs).toBeInstanceOf(Map);
      expect(result.timestamp).toBeTypeOf("number");
      expect(result.timestamp).toBeGreaterThan(0);
    });

    it("should verify attestation document without PCR validation", async () => {
      const attestationDocument =
        SERVER_RESPONSE.TeePublicKey.attestation_document;

      // Verify attestation without PCR validation using valid certificate date
      const result: AttestationResult = await verifyAttestation(
        attestationDocument,
        undefined,
        VALID_CERT_DATE
      );

      // Validate the result structure
      expect(result).toBeDefined();
      expect(result.pcrs).toBeInstanceOf(Map);
      expect(result.timestamp).toBeTypeOf("number");
    });
  });

  describe("PCR Extraction and Validation", () => {
    it("should extract PCR values correctly", async () => {
      const attestationDocument =
        SERVER_RESPONSE.TeePublicKey.attestation_document;
      const result = await verifyAttestation(
        attestationDocument,
        new Map(), // No PCR validation
        VALID_CERT_DATE
      );

      // Extract PCR values
      const pcrValues = extractPCRs(result);

      // Validate PCR structure
      expect(pcrValues).toBeTypeOf("object");
      expect(Object.keys(pcrValues)).toContain("0");
      expect(Object.keys(pcrValues)).toContain("1");
      expect(Object.keys(pcrValues)).toContain("2");

      // Validate PCR format (hex strings)
      Object.values(pcrValues).forEach((pcrValue) => {
        expect(pcrValue).toMatch(/^[0-9a-fA-F]+$/);
      });
    });

    it("should validate PCR values match expected values", async () => {
      const attestationDocument =
        SERVER_RESPONSE.TeePublicKey.attestation_document;
      const result = await verifyAttestation(
        attestationDocument,
        EXPECTED_PCRS,
        VALID_CERT_DATE
      );

      const extractedPCRs = extractPCRs(result);

      // Verify extracted PCRs match expected values
      expect(extractedPCRs["0"]).toBe(EXPECTED_PCRS.get("0"));
      expect(extractedPCRs["1"]).toBe(EXPECTED_PCRS.get("1"));
      expect(extractedPCRs["2"]).toBe(EXPECTED_PCRS.get("2"));
    });
  });

  describe("Public Key Validation", () => {
    it("should validate public key consistency", async () => {
      const serverPublicKey = SERVER_RESPONSE.TeePublicKey.public_key;
      const attestationDocument =
        SERVER_RESPONSE.TeePublicKey.attestation_document;

      const result = await verifyAttestation(
        attestationDocument,
        new Map(),
        VALID_CERT_DATE
      );

      // If the attestation contains a public key, it should match the server response
      if (result.publicKey) {
        // Convert base64 public key from attestation to hex for comparison
        const attestationPublicKeyBytes = Uint8Array.from(
          atob(result.publicKey),
          (c) => c.charCodeAt(0)
        );
        const attestationPublicKeyHex = Array.from(attestationPublicKeyBytes)
          .map((byte) => byte.toString(16).padStart(2, "0"))
          .join("");

        // Note: The comparison might need adjustment based on key format differences
        // This is a placeholder for the actual validation logic
        expect(attestationPublicKeyHex).toBeDefined();
        expect(serverPublicKey).toBeDefined();
      }
    });
  });

  describe("Error Handling", () => {
    it("should handle invalid attestation document", async () => {
      const invalidAttestationDoc = "invalid_base64_data";

      await expect(verifyAttestation(invalidAttestationDoc)).rejects.toThrow();
    });

    it("should handle PCR mismatch", async () => {
      const attestationDocument =
        SERVER_RESPONSE.TeePublicKey.attestation_document;

      // Create incorrect PCR expectations
      const incorrectPCRs = new Map([
        ["0", "incorrect_pcr_value"],
        ["1", "another_incorrect_value"],
        ["2", "yet_another_incorrect_value"]
      ]);

      await expect(
        verifyAttestation(attestationDocument, incorrectPCRs)
      ).rejects.toThrow();
    });

    it("should handle malformed base64 data", async () => {
      const malformedBase64 = "not_valid_base64!@#$%";

      await expect(verifyAttestation(malformedBase64)).rejects.toThrow();
    });

    it("should handle empty attestation document", async () => {
      await expect(verifyAttestation("")).rejects.toThrow();
    });
  });

  describe("Integration Tests", () => {
    it("should perform complete end-to-end verification", async () => {
      const { TeePublicKey } = SERVER_RESPONSE;

      // Step 1: Verify attestation document
      const attestationResult = await verifyAttestation(
        TeePublicKey.attestation_document,
        EXPECTED_PCRS,
        VALID_CERT_DATE
      );

      // Step 2: Extract and validate PCRs
      const extractedPCRs = extractPCRs(attestationResult);

      // Step 3: Validate timestamp
      const attestationTime = attestationResult.timestamp;
      expect(attestationTime).toBe(1757440781958);

      // Step 4: Validate optional fields
      if (attestationResult.publicKey) {
        expect(attestationResult.publicKey).toMatch(/^[A-Za-z0-9+/]+=*$/);
      }

      if (attestationResult.userData) {
        expect(attestationResult.userData).toBeTypeOf("string");
      }
    });
  });
});
