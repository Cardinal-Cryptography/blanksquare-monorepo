#[cfg(not(feature = "without_attestation"))]
use aws_ne_sys as ne;
use rand::RngCore;
use rsa::{pkcs1::DecodeRsaPublicKey, pkcs8::DecodePublicKey, sha2::Sha256, Oaep, RsaPublicKey};
use shielder_scheduler_common::vsock::VsockError;

#[cfg(not(feature = "without_attestation"))]
pub struct KmsCrypto {
    region: String,
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
}

#[cfg(feature = "without_attestation")]
pub struct KmsCrypto {
    private_key: Vec<u8>,
}

impl KmsCrypto {
    /// Create a new KMS crypto helper using the provided KMS KeyId/ARN.
    #[cfg(not(feature = "without_attestation"))]
    pub fn new(
        region: String,
        access_key: String,
        secret_key: String,
        session_token: Option<String>,
    ) -> Result<Self, VsockError> {
        Ok(Self {
            region,
            access_key,
            secret_key,
            session_token,
        })
    }

    #[cfg(feature = "without_attestation")]
    pub fn new(private_key: Vec<u8>) -> Result<Self, VsockError> {
        Ok(Self { private_key })
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, VsockError> {
        #[cfg(not(feature = "without_attestation"))]
        let decrypted_data = ne::kms_decrypt(
            self.region.as_bytes(),
            self.access_key.as_bytes(),
            self.secret_key.as_bytes(),
            self.session_token.as_deref().unwrap_or("").as_bytes(),
            ciphertext,
        )
        .map_err(|e| VsockError::KMS(format!("kms_decrypt error: {e:?}")))?;
        #[cfg(feature = "without_attestation")]
        let decrypted_data = {
            use rsa::{pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey, RsaPrivateKey};
            let private_key = RsaPrivateKey::from_pkcs1_der(&self.private_key)
                .or_else(|_| RsaPrivateKey::from_pkcs8_der(&self.private_key))
                .map_err(|e| VsockError::KMS(format!("Failed to parse private key: {e:?}")))?;

            let padding = Oaep::new::<Sha256>();
            private_key.decrypt(padding, ciphertext).map_err(|e| {
                VsockError::KMS(format!("Failed to decrypt with private key: {e:?}"))
            })?
        };
        Ok(decrypted_data)
    }

    pub fn verify_public_key(&self, public_key: &[u8]) -> Result<(), VsockError> {
        // 1. generate random bytes
        let mut rng = rand::thread_rng();
        let mut random_bytes = vec![0u8; 32];
        rng.fill_bytes(&mut random_bytes);

        // 2. Parse the public key (assuming DER format)
        let rsa_public_key = RsaPublicKey::from_pkcs1_der(public_key)
            .or_else(|_| RsaPublicKey::from_public_key_der(public_key))
            .map_err(|e| VsockError::KMS(format!("Failed to parse public key: {e:?}")))?;

        // 3. Encrypt the random bytes with the public key using OAEP-SHA256 padding
        let padding = Oaep::new::<Sha256>();
        let encrypted_data = rsa_public_key
            .encrypt(&mut rng, padding, &random_bytes)
            .map_err(|e| VsockError::KMS(format!("Failed to encrypt with public key: {e:?}")))?;

        // 4. Decrypt using KMS
        let decrypted_data = self.decrypt(&encrypted_data)?;

        // 5. Compare the original random bytes with the decrypted data
        if random_bytes == decrypted_data {
            Ok(())
        } else {
            Err(VsockError::KMS(
                "Public key verification failed: decrypted data does not match original"
                    .to_string(),
            ))
        }
    }
}
