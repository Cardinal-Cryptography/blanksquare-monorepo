import { encryptPadded } from "@cardinal-cryptography/ecies-encryption-lib";
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
    const encryptedData = await encryptPadded(
      new TextEncoder().encode(this.referralId),
      publicKey,
      referralPaddedLength
    );
    return encryptedData;
  }
}
