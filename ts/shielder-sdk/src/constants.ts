export const contractVersion = "0x000101";
export const relayPath = "/relay";
export const feePath = "/quote_fees";
export const feeAddressPath = "/fee_address";
/**
 * Gas limit for shield action: newAccount or deposit
 * Originally they have ~2M gas limit, but we set it to 3M to be safe
 */
export const storageSchemaVersion = 2;
export const nativeTokenAddress = "0x0000000000000000000000000000000000000000";

export const firstAccountIndex = 0;

/**
 * Length of the referral ID padding in bytes.
 * This is used to ensure that the referral ID is padded to a fixed length for encryption.
 * 4 bytes for the length prefix + 16 bytes for padded data.
 */
export const referralPaddedLength = 20;
