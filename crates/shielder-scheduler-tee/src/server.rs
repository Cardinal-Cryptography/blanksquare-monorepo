use std::sync::Arc;

use alloy_primitives::{Address, U256};
#[cfg(not(feature = "without_attestation"))]
use aws_nitro_enclaves_nsm_api::{
    api::Request as NsmRequest,
    api::Response as NsmResponse,
    driver::{nsm_exit, nsm_init, nsm_process_request},
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use log::{debug, info};
use shielder_scheduler_common::{
    protocol::{
        EncryptionEnvelope, MerklePath, Payload, RelayCalldata, Request, Response, TEEServer,
    },
    vsock::VsockError,
};
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};

use crate::{command_line_args::CommandLineArgs, kms::KmsCrypto};

pub struct Server {
    kms: KmsCrypto,
    #[cfg(not(feature = "without_attestation"))]
    nsm_fd: i32,

    listener: VsockListener,
}

impl Server {
    /// Initialize the TEE server:
    /// - Binds vsock listener on the provided port
    /// - Initializes the Nitro Enclaves NSM driver (unless without_attestation)
    /// - Creates the KMS client (unless without_attestation)
    pub async fn new(options: CommandLineArgs) -> Result<Arc<Self>, VsockError> {
        #[cfg(feature = "without_attestation")]
        info!("Running server without attestation (TEST BUILD).");

        let address = VsockAddr::new(options.tee_cid, options.tee_port);
        let listener = VsockListener::bind(address)?;

        #[cfg(not(feature = "without_attestation"))]
        let nsm_fd = Self::init_nsm_driver()?;

        let public_key = BASE64.decode(options.kms_public_key.as_bytes()).map_err(|e| {
            VsockError::Protocol(format!("Failed to decode KMS public key from base64: {e:?}"))
        })?;


        let kms = KmsCrypto::new(
            public_key,
            options.kms_key_id,
            options.kms_region,
            options.kms_encryption_algorithm,
            #[cfg(feature = "without_attestation")]
            BASE64.decode(options.private_key.as_bytes()).map_err(|e| {
                VsockError::Protocol(format!("Failed to decode private key from base64: {e:?}"))
            })?,
        )
        .map_err(|e| VsockError::Protocol(format!("KMS init error: {e}")))?;

        Ok(Arc::new(Self {
            listener,
            kms,
            #[cfg(not(feature = "without_attestation"))]
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
                        Request::TeePublicKey => {
                            self.public_key_response().await
                        }
                        Request::PrepareRelayCalldata {
                            encryption_envelope,
                            relayer_address,
                            relayer_fee,
                            merkle_path,
                        } => {
                            self.prepare_relay_calldata_response(
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
    async fn public_key_response(&self) -> Result<Response, VsockError> {
        #[cfg(not(feature = "without_attestation"))]
        let attestation_document = self.request_attestation_from_nsm_driver(self.kms.kms_public_key.clone())?;

        #[cfg(feature = "without_attestation")]
        let attestation_document = Vec::new();

        Ok(Response::TeePublicKey {
            public_key: self.kms.kms_public_key.clone(),
            attestation_document,
        })
    }

    async fn prepare_relay_calldata_response(
        &self,
        encryption_envelope: EncryptionEnvelope,
        _relayer_address: Address,
        _relayer_fee: U256,
        _merkle_path: MerklePath,
    ) -> Result<Response, VsockError> {
        let decrypted_payload = self.kms.decrypt_payload(&encryption_envelope)?;

        let deserialized_payload: Payload = serde_json::from_slice(&decrypted_payload).map_err(|e| {
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

    #[cfg(not(feature = "without_attestation"))]
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

    #[cfg(not(feature = "without_attestation"))]
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

#[cfg(not(feature = "without_attestation"))]
impl Drop for Server {
    fn drop(&mut self) {
        info!("Closing file descriptor to /dev/nsm driver.");
        nsm_exit(self.nsm_fd);
    }
}
