#[cfg(not(feature = "without_attestation"))]
use std::process::{Command, Stdio};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
#[cfg(not(feature = "without_attestation"))]
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
#[cfg(feature = "without_attestation")]
use rsa::{pkcs8::EncodePublicKey, sha2::Sha256, Oaep, RsaPrivateKey, RsaPublicKey};
use shielder_scheduler_common::{protocol::EncryptionEnvelope, vsock::VsockError};

#[cfg(not(feature = "without_attestation"))]
#[derive(serde::Deserialize)]
struct KmsPublicKeyResponse {
    #[serde(rename = "PublicKey")]
    public_key: String,
}

#[cfg(not(feature = "without_attestation"))]
pub struct KmsCrypto {
    kms_key_id: String,
    kms_region: String,
    kms_encryption_algorithm: String,
}

#[cfg(feature = "without_attestation")]
pub struct KmsCrypto {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
}

impl KmsCrypto {
    /// Create a new KMS crypto helper using the provided KMS KeyId/ARN.
    #[cfg(not(feature = "without_attestation"))]
    pub fn new(
        kms_key_id: String,
        kms_region: String,
        kms_encryption_algorithm: String,
    ) -> Result<Self, VsockError> {
        Ok(Self {
            kms_key_id,
            kms_region,
            kms_encryption_algorithm,
        })
    }

    #[cfg(feature = "without_attestation")]
    pub fn new() -> Result<Self, VsockError> {
        let private_key = RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
            .map_err(|e| VsockError::KMS(format!("Failed to generate private key: {e:?}")))?;
        let public_key = RsaPublicKey::from(&private_key);
        Ok(Self {
            private_key,
            public_key,
        })
    }

    fn decrypt_dek(&self, encrypted_dek: &[u8]) -> Result<Vec<u8>, VsockError> {
        #[cfg(not(feature = "without_attestation"))]
        let decrypted_data = {
            let output = Command::new("/usr/bin/kmstool_enclave_cli")
                .arg("decrypt")
                .arg(format!("--region={}", self.kms_region))
                .arg(format!("--key-id={}", self.kms_key_id))
                .arg(format!("--ciphertext={}", BASE64.encode(encrypted_dek)))
                .arg(format!(
                    "--encryption-algorithm={}",
                    self.kms_encryption_algorithm
                ))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .map_err(|e| {
                    VsockError::KMS(format!("failed to run kmstool_enclave_cli: {e:?}"))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(VsockError::KMS(format!("kmstool failed: {}", stderr)));
            }

            let b64_str = String::from_utf8(output.stdout)
                .map_err(|_| VsockError::KMS("stdout not valid utf8".into()))?;
            BASE64
                .decode(b64_str.trim())
                .map_err(|_| VsockError::KMS("base64 decode failed".into()))?
        };

        #[cfg(feature = "without_attestation")]
        let decrypted_data = {
            let padding = Oaep::new::<Sha256>();
            self.private_key
                .decrypt(padding, encrypted_dek)
                .map_err(|e| {
                    VsockError::KMS(format!("Failed to decrypt with private key: {e:?}"))
                })?
        };
        Ok(decrypted_data)
    }

    pub fn decrypt_payload(
        &self,
        encryption_envelope: &EncryptionEnvelope,
    ) -> Result<Vec<u8>, VsockError> {
        let dek = self.decrypt_dek(&encryption_envelope.encrypted_dek)?;

        if dek.len() != 32 {
            return Err(VsockError::KMS(format!(
                "Invalid data key length: expected 32 bytes, got {}",
                dek.len()
            )));
        }

        let mut full_ciphertext = encryption_envelope.encrypted_payload.clone();
        full_ciphertext.extend(encryption_envelope.auth_tag.clone());

        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| VsockError::KMS(format!("Failed to create AES cipher: {e:?}")))?;

        let nonce = Nonce::from_slice(&encryption_envelope.iv);
        let decrypted_payload = cipher
            .decrypt(nonce, full_ciphertext.as_ref())
            .map_err(|e| VsockError::KMS(format!("Failed to decrypt payload: {e:?}")))?;

        Ok(decrypted_payload)
    }

    pub fn public_key(&self) -> Result<Vec<u8>, VsockError> {
        #[cfg(not(feature = "without_attestation"))]
        let public_key = {
            let output = Command::new("/usr/bin/kmstool_enclave_cli")
                .arg("get-public-key")
                .arg(format!("--region={}", self.kms_region))
                .arg(format!("--key-id={}", self.kms_key_id))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .map_err(|e| {
                    VsockError::KMS(format!("failed to run kmstool_enclave_cli: {e:?}"))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(VsockError::KMS(format!("kmstool failed: {}", stderr)));
            }

            // Parse JSON
            let resp: KmsPublicKeyResponse = serde_json::from_slice(&output.stdout)
                .map_err(|_| VsockError::KMS("parse kmstool JSON failed".into()))?;

            BASE64
                .decode(resp.public_key.trim())
                .map_err(|_| VsockError::KMS("base64 decode failed".into()))?
        };

        #[cfg(feature = "without_attestation")]
        let public_key = self
            .public_key
            .to_public_key_der()
            .map_err(|e| VsockError::KMS(format!("Failed to encode public key to DER: {e:?}")))?
            .to_vec();
        Ok(public_key)
    }
}
