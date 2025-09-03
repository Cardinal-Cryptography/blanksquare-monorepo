use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct CommandLineArgs {
    /// Internal port on which host and tee applications talks to each other
    /// This is the part of the vsock endpoint, which is tee_cid:tee_port
    #[clap(short, long, default_value_t = shielder_scheduler_common::protocol::VSOCK_PORT, env = "TEE_PORT")]
    pub tee_port: u32,

    /// A context identifier on which this server and TEE server communicate with each other
    /// This is the part of the vsock endpoint, which is tee_cid:tee_port
    #[clap(long, default_value_t = shielder_scheduler_common::protocol::VMADDR_CID_ANY, env = "TEE_CID")]
    pub tee_cid: u32,

    #[cfg(not(feature = "without_attestation"))]
    #[clap(
        long,
        default_value = "arn:aws:kms:eu-west-1:381491925369:key/7ff40184-1c5b-4aa1-8718-18ef89815663",
        env = "KMS_KEY_ID"
    )]
    pub kms_key_id: String,

    #[cfg(not(feature = "without_attestation"))]
    #[clap(long, default_value = "eu-west-1", env = "KMS_REGION")]
    pub kms_region: String,

    #[cfg(not(feature = "without_attestation"))]
    #[clap(
        long,
        default_value = "RSAES_OAEP_SHA_256",
        env = "KMS_ENCRYPTION_ALGORITHM"
    )]
    pub kms_encryption_algorithm: String,
}
