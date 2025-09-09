/**
 * Utility functions for AWS Nitro Enclaves attestation verification
 */

/**
 * Converts a Uint8Array to a base64 string
 * @param bytes - The bytes to convert
 * @returns The base64 string
 */
export function bytesToBase64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes));
}

/**
 * Converts a base64 string to a Uint8Array
 * @param base64 - The base64 string to convert
 * @returns The Uint8Array
 */
export function base64ToBytes(base64: string): Uint8Array {
  const binaryString = atob(base64);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

/**
 * Converts a Uint8Array to a hexadecimal string
 * @param uint8 - The Uint8Array to convert
 * @returns The hexadecimal string
 */
export function uint8ToHex(uint8: Uint8Array): string {
  return Array.from(uint8)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Converts a hexadecimal string to a Uint8Array
 * @param hex - The hexadecimal string to convert
 * @returns The Uint8Array
 */
export function hexToUint8(hex: string): Uint8Array {
  if (hex.length % 2 !== 0) {
    throw new Error("Hex string must have an even length");
  }
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes;
}
