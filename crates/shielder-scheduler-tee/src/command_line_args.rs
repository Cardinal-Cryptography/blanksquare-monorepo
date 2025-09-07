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
    
    #[clap(long, default_value_t = 8000, env = "KMS_PROXY_PORT")]
    pub kms_proxy_port: u32,

    #[cfg(feature = "without_attestation")]
    #[clap(long, env = "PRIVATE_KEY")]
    pub private_key: String,
}
