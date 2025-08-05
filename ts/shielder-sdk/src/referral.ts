import {
  encryptPadded,
  getCrypto,
  fromHex
} from "@cardinal-cryptography/ecies-encryption-lib";

type Hex = `0x${string}`;

export class Referral {
  referralId: string;
  encryptionPublicKey: () => Promise<Hex>;
  constructor(referralId: string, encryptionPublicKey: () => Promise<Hex>) {
    this.referralId = referralId;
    this.encryptionPublicKey = encryptionPublicKey;
  }

  public async getEncryptedReferral(): Promise<Uint8Array> {
    const publicKey = await this.encryptionPublicKey();
    // 4 bytes for the length of the referralId
    // followed by the referralId itself
    const dataLength = new TextEncoder().encode(this.referralId).length + 4;
    // paddedLength is at least 16 bytes
    // If referralDataLength is greater than 16, we use the next power of 2
    // that is greater than or equal to referralDataLength
    const paddedLength =
      dataLength < 16 ? 16 : 2 ** Math.ceil(Math.log2(dataLength));
    const crypto = await getCrypto();
    const encryptedData = await encryptPadded(
      this.referralId,
      publicKey,
      crypto,
      paddedLength
    );
    return fromHex(encryptedData);
  }
}
