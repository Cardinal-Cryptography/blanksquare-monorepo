#[cfg(not(feature = "local-run"))]
use std::process::{Command, Stdio};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
#[cfg(feature = "local-run")]
use log::info;
#[cfg(not(feature = "local-run"))]
use log::{debug, info};
#[cfg(feature = "local-run")]
use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
use rsa::{pkcs8::DecodePublicKey, sha2::Sha256, Oaep, RsaPublicKey};
use shielder_scheduler_common::{
    protocol::{AwsConfig, EncryptionEnvelope},
    vsock::VsockError,
};

pub struct KmsCrypto {
    #[cfg(not(feature = "local-run"))]
    kms_proxy_port: u32,
    #[cfg(feature = "local-run")]
    private_key: Vec<u8>,
}

impl KmsCrypto {
    pub fn new(
        #[cfg(not(feature = "local-run"))] kms_proxy_port: u32,
        #[cfg(feature = "local-run")] _kms_proxy_port: u32,
        #[cfg(feature = "local-run")] private_key: String,
    ) -> Result<Self, VsockError> {
        Ok(Self {
            #[cfg(not(feature = "local-run"))]
            kms_proxy_port,
            #[cfg(feature = "local-run")]
            private_key: BASE64.decode(private_key).map_err(|e| {
                VsockError::KMS(format!(
                    "Failed to decode PRIVATE_KEY_BASE64 from base64: {e:?}"
                ))
            })?,
        })
    }

    fn decrypt_dek(
        &self,
        #[cfg(not(feature = "local-run"))] aws_config: &AwsConfig,
        #[cfg(feature = "local-run")] _aws_config: &AwsConfig,
        encrypted_dek: &[u8],
    ) -> Result<Vec<u8>, VsockError> {
        #[cfg(not(feature = "local-run"))]
        let decrypted_data = {
            info!("Decrypting data encryption key (DEK)");
            let mut cmd = Command::new("/usr/local/bin/kmstool_enclave_cli");
            cmd.arg("decrypt")
                .arg("--region")
                .arg(&aws_config.aws_region)
                .arg("--proxy-port")
                .arg(self.kms_proxy_port.to_string())
                .arg("--aws-access-key-id")
                .arg(&aws_config.aws_access_key_id)
                .arg("--aws-secret-access-key")
                .arg(&aws_config.aws_secret_access_key)
                .arg("--aws-session-token")
                .arg(&aws_config.aws_session_token)
                .arg("--key-id")
                .arg(&aws_config.kms_key_id)
                .arg("--ciphertext")
                .arg(BASE64.encode(encrypted_dek))
                .arg("--encryption-algorithm")
                .arg(&aws_config.kms_encryption_algorithm)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            debug!("Executing command: {:?}", cmd);
            let output = cmd.output().map_err(|e| {
                VsockError::KMS(format!("failed to run kmstool_enclave_cli: {e:?}"))
            })?;

            if !output.status.success() {
                let stderr: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&output.stderr);
                info!("Decrypting data encryption key failed with {stderr}");
                return Err(VsockError::KMS(format!("kmstool failed: {}", stderr)));
            }
            info!("Decrypting data encryption key success");

            let stdout_str = String::from_utf8(output.stdout)
                .map_err(|_| VsockError::KMS("stdout not valid utf8".into()))?;

            // Parse the output - it should have format "PLAINTEXT: <base64-encoded-data>"
            let plaintext_line = stdout_str
                .lines()
                .find(|line| line.starts_with("PLAINTEXT: "))
                .ok_or_else(|| {
                    VsockError::KMS("No PLAINTEXT line found in kmstool output".into())
                })?;

            let b64_str = plaintext_line
                .strip_prefix("PLAINTEXT: ")
                .ok_or_else(|| VsockError::KMS("Invalid PLAINTEXT format".into()))?;

            info!("Parsing kmstool output success");
            BASE64
                .decode(b64_str.trim())
                .map_err(|_| VsockError::KMS("base64 decode failed".into()))?
        };

        #[cfg(feature = "local-run")]
        let decrypted_data = {
            let padding = Oaep::new::<Sha256>();
            let private_key = RsaPrivateKey::from_pkcs8_der(&self.private_key).map_err(|e| {
                VsockError::KMS(format!("Failed to parse private key from DER: {e:?}"))
            })?;
            private_key.decrypt(padding, encrypted_dek).map_err(|e| {
                VsockError::KMS(format!("Failed to decrypt with private key: {e:?}"))
            })?
        };
        Ok(decrypted_data)
    }

    pub fn decrypt_payload(
        &self,
        aws_config: &AwsConfig,
        encryption_envelope: &EncryptionEnvelope,
    ) -> Result<Vec<u8>, VsockError> {
        info!("Decrypting DEK and payload");
        let dek = self.decrypt_dek(aws_config, &encryption_envelope.encrypted_dek)?;
        info!("Decrypting data encryption key success");

        if dek.len() != 32 {
            return Err(VsockError::KMS(format!(
                "Invalid data key length: expected 32 bytes, got {}",
                dek.len()
            )));
        }

        info!("Decrypting payload");

        // Validate IV/nonce length - AES-GCM requires exactly 12 bytes (96 bits)
        if encryption_envelope.iv.len() != 12 {
            return Err(VsockError::KMS(format!(
                "Invalid IV length: expected 12 bytes for AES-GCM, got {}",
                encryption_envelope.iv.len()
            )));
        }

        // Validate auth tag length - AES-GCM typically uses 16 bytes (128 bits)
        if encryption_envelope.auth_tag.len() != 16 {
            return Err(VsockError::KMS(format!(
                "Invalid auth tag length: expected 16 bytes for AES-GCM, got {}",
                encryption_envelope.auth_tag.len()
            )));
        }

        // Validate that we have some ciphertext
        if encryption_envelope.encrypted_payload.is_empty() {
            return Err(VsockError::KMS(
                "Invalid encrypted payload: cannot be empty".into(),
            ));
        }

        let mut full_ciphertext = encryption_envelope.encrypted_payload.clone();
        full_ciphertext.extend(encryption_envelope.auth_tag.clone());

        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| VsockError::KMS(format!("Failed to create AES cipher: {e:?}")))?;

        let nonce = Nonce::from_slice(&encryption_envelope.iv);
        let decrypted_payload = cipher
            .decrypt(nonce, full_ciphertext.as_ref())
            .map_err(|e| VsockError::KMS(format!("Failed to decrypt payload: {e:?}")))?;
        info!("Decrypting payloadsuccess");

        Ok(decrypted_payload)
    }

    pub fn verify_public_key(&self, aws_config: &AwsConfig) -> Result<(), VsockError> {
        let expected_data = "This is a correct key".to_string();

        // Parse public key from DER bytes (the public_key field now contains base64-decoded DER bytes)
        let oaep_padding = Oaep::new::<Sha256>();
        let encrypted_data: Vec<u8> = RsaPublicKey::from_public_key_der(&aws_config.public_key)
            .map_err(|e| VsockError::KMS(format!("Failed to parse public key DER: {e:?}")))?
            .encrypt(
                &mut rand::thread_rng(),
                oaep_padding,
                expected_data.as_bytes(),
            )
            .map_err(|e| VsockError::KMS(format!("Failed to encrypt data: {e:?}")))?;

        let decrypted_data = self.decrypt_dek(aws_config, &encrypted_data)?;
        if decrypted_data != expected_data.as_bytes() {
            return Err(VsockError::KMS(
                "Public key verification failed: decrypted data does not match expected".into(),
            ));
        }
        info!("Public key verification succeeded");
        Ok(())
    }
}
