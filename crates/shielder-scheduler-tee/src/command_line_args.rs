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

    #[cfg(feature = "without_attestation")]
    #[clap(long, env = "PRIVATE_KEY")]
    pub private_key: String,

    #[cfg(not(feature = "without_attestation"))]
    #[clap(long, default_value = "", env = "AWS_REGION")]
    pub kms_region: String,

    #[cfg(not(feature = "without_attestation"))]
    #[clap(long, default_value = "", env = "AWS_ACCESS_KEY_ID")]
    pub kms_access_key: String,

    #[cfg(not(feature = "without_attestation"))]
    #[clap(long, default_value = "", env = "AWS_SECRET_ACCESS_KEY")]
    pub kms_secret_key: String,

    #[cfg(not(feature = "without_attestation"))]
    #[clap(long, default_value = None, env = "AWS_SESSION_TOKEN")]
    pub kms_session_token: Option<String>,
}
