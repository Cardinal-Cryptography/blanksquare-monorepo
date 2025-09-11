use std::sync::Arc;

use alloy_primitives::{Address, U256};
#[cfg(not(feature = "local-run"))]
use aws_nitro_enclaves_nsm_api::{
    api::Request as NsmRequest,
    api::Response as NsmResponse,
    driver::{nsm_exit, nsm_init, nsm_process_request},
};
use log::{debug, info};
use shielder_scheduler_common::{
    protocol::{
        AwsConfig, EncryptionEnvelope, MerklePath, Payload, RelayCalldata, Request, Response,
        TEEServer,
    },
    vsock::VsockError,
};
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};

use crate::{command_line_args::CommandLineArgs, kms::KmsCrypto};

pub struct Server {
    kms: KmsCrypto,
    #[cfg(not(feature = "local-run"))]
    nsm_fd: i32,

    listener: VsockListener,
}

impl Server {
    pub async fn new(options: CommandLineArgs) -> Result<Arc<Self>, VsockError> {
        #[cfg(feature = "local-run")]
        info!("Running server without attestation (TEST BUILD).");

        let address = VsockAddr::new(options.tee_cid, options.tee_port);
        let listener = VsockListener::bind(address)?;

        #[cfg(not(feature = "local-run"))]
        let nsm_fd = Self::init_nsm_driver()?;

        let kms = KmsCrypto::new(
            options.kms_proxy_port,
            #[cfg(feature = "local-run")]
            options.private_key,
        );

        Ok(Arc::new(Self {
            listener,
            kms,
            #[cfg(not(feature = "local-run"))]
            nsm_fd,
        }))
    }

    pub fn local_addr(&self) -> Result<VsockAddr, VsockError> {
        Ok(self.listener.local_addr()?)
    }

    pub fn listener(&self) -> &VsockListener {
        &self.listener
    }

    pub async fn handle_client(self: Arc<Self>, stream: VsockStream) {
        let result = self.do_handle_client(stream).await;
        debug!("Client disconnected: {result:?}");
    }

    /// Accept and serve a single vsock client connection, handling requests in a loop.
    async fn do_handle_client(&self, stream: VsockStream) -> Result<(), VsockError> {
        let mut server: TEEServer = stream.into();

        loop {
            server
                .handle_request(|request| async move {
                    match request {
                        Request::Ping => Ok(Response::Pong),
                        Request::TeePublicKey { aws_config } => {
                            self.public_key_response(&aws_config).await
                        }
                        Request::PrepareRelayCalldata {
                            aws_config,
                            encryption_envelope,
                            relayer_address,
                            relayer_fee,
                            merkle_path,
                        } => {
                            self.prepare_relay_calldata_response(
                                &aws_config,
                                encryption_envelope,
                                relayer_address,
                                relayer_fee,
                                merkle_path,
                            )
                            .await
                        }
                    }
                })
                .await?;
        }
    }

    /// Return the public key (base64) and
    /// an attestation document that embeds the same public key.
    async fn public_key_response(&self, aws_config: &AwsConfig) -> Result<Response, VsockError> {
        self.kms.verify_public_key(aws_config)?;

        #[cfg(not(feature = "local-run"))]
        let attestation_document =
            self.request_attestation_from_nsm_driver(aws_config.public_key.clone())?;

        #[cfg(feature = "local-run")]
        let attestation_document = Vec::new();

        Ok(Response::TeePublicKey {
            public_key: aws_config.public_key.clone(),
            attestation_document,
        })
    }

    async fn prepare_relay_calldata_response(
        &self,
        aws_config: &AwsConfig,
        encryption_envelope: EncryptionEnvelope,
        _relayer_address: Address,
        _relayer_fee: U256,
        _merkle_path: MerklePath,
    ) -> Result<Response, VsockError> {
        let decrypted_payload = self.kms.decrypt_payload(aws_config, &encryption_envelope)?;

        let deserialized_payload: Payload =
            serde_json::from_slice(&decrypted_payload).map_err(|e| {
                VsockError::Protocol(format!("Failed to deserialize decrypted payload: {e:?}"))
            })?;

        // TODO: Implement proof generation logic here
        info!("Received payload: {:?}", deserialized_payload);

        Ok(Response::PrepareRelayCalldata {
            calldata: RelayCalldata {
                expected_contract_version: [0, 0, 0].into(),
                amount: deserialized_payload.withdrawal_value,
                withdraw_address: Address::random(),
                merkle_root: U256::from(0),
                nullifier_hash: U256::from(0),
                new_note: U256::from(0),
                proof: Vec::new().into(),
                fee_token: Default::default(),
                fee_amount: U256::from(0),
                mac_salt: U256::from(0),
                mac_commitment: U256::from(0),
                pocket_money: U256::from(0),
                memo: Vec::new().into(),
            },
        }) // Placeholder response
    }

    #[cfg(not(feature = "local-run"))]
    fn request_attestation_from_nsm_driver(
        &self,
        tee_public_key: Vec<u8>,
    ) -> Result<Vec<u8>, VsockError> {
        match nsm_process_request(
            self.nsm_fd,
            NsmRequest::Attestation {
                user_data: None,
                public_key: Some(tee_public_key.into()),
                nonce: None,
            },
        ) {
            NsmResponse::Attestation { document } => Ok(document),
            _ => Err(VsockError::Protocol(String::from(
                "NSM driver failed to compute attestation.",
            ))),
        }
    }

    #[cfg(not(feature = "local-run"))]
    fn init_nsm_driver() -> Result<i32, VsockError> {
        info!("Opening file descriptor to /dev/nsm driver.");
        let nsm_fd = nsm_init();

        if nsm_fd < 0 {
            return Err(VsockError::Protocol(String::from(
                "Failed to initialize NSM driver.",
            )));
        }

        Ok(nsm_fd)
    }
}

#[cfg(not(feature = "local-run"))]
impl Drop for Server {
    fn drop(&mut self) {
        info!("Closing file descriptor to /dev/nsm driver.");
        nsm_exit(self.nsm_fd);
    }
}
