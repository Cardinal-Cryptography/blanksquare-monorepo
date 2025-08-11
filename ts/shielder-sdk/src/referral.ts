import {
  encryptPadded,
  getCrypto,
  fromHex
} from "@cardinal-cryptography/ecies-encryption-lib";
import { referralPaddedLength } from "./constants";

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
    const crypto = await getCrypto();
    const encryptedData = await encryptPadded(
      this.referralId,
      publicKey,
      crypto,
      referralPaddedLength
    );
    return fromHex(encryptedData);
  }
}
