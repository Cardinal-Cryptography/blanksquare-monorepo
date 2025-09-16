use std::sync::Arc;

use alloy_primitives::{Address, U256};
#[cfg(not(feature = "local-run"))]
use aws_nitro_enclaves_nsm_api::{
    api::Request as NsmRequest,
    api::Response as NsmResponse,
    driver::{nsm_exit, nsm_init, nsm_process_request},
};
use chrono::Utc;
use log::{debug, info};
use shielder_scheduler_common::{
    protocol::{AwsConfig, EncryptionEnvelope, Payload, Request, Response, TEEServer},
    vsock::VsockError,
};
use shielder_setup::consts::{ARITY, TREE_HEIGHT};
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};

use crate::{
    command_line_args::CommandLineArgs, kms::KmsDecryptionController, withdraw::WithdrawCircuit,
};

struct RelayParams {
    relayer_address: Address,
    relayer_fee: U256,
    merkle_path: Box<[[U256; ARITY]; TREE_HEIGHT]>,
    merkle_root: U256,
}

pub struct Server {
    kms: KmsDecryptionController,
    #[cfg(not(feature = "local-run"))]
    nsm_fd: i32,

    listener: VsockListener,
}

impl Server {
    pub async fn new(options: CommandLineArgs) -> Result<Arc<Self>, VsockError> {
        #[cfg(feature = "local-run")]
        info!("local-run: attestation disabled; using locally provided private key (TEST BUILD).");

        let address = VsockAddr::new(options.tee_cid, options.tee_port);
        let listener = VsockListener::bind(address)?;

        #[cfg(not(feature = "local-run"))]
        let nsm_fd = Self::init_nsm_driver()?;

        let kms = KmsDecryptionController::new(
            options.kms_proxy_port,
            #[cfg(feature = "local-run")]
            options.private_key,
        )?;

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
                            merkle_root,
                        } => {
                            let relay_params = RelayParams {
                                relayer_address,
                                relayer_fee,
                                merkle_path,
                                merkle_root,
                            };
                            self.prepare_relay_calldata_response(
                                &aws_config,
                                *encryption_envelope,
                                relay_params,
                            )
                            .await
                        }
                    }
                })
                .await?;
        }
    }

    /// Return the KMS public key (base64) and an attestation document embedding the same key.
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
        relay_params: RelayParams,
    ) -> Result<Response, VsockError> {
        let decrypted_payload = self.kms.decrypt_payload(aws_config, &encryption_envelope)?;

        let payload: Payload =
            serde_json::from_slice(&decrypted_payload).map_err(|e| {
                VsockError::Protocol(format!("Failed to deserialize decrypted payload: {e:?}"))
            })?;

        if payload.relay_after > Utc::now().timestamp() {
            return Err(VsockError::Protocol(format!(
                "Request scheduled for future timestamp: {}",
                payload.relay_after
            )));
        }

        if relay_params.relayer_fee > payload.max_relayer_fee {
            return Err(VsockError::Protocol(format!(
                "Relayer fee {} exceeds max relayer fee {}",
                relay_params.relayer_fee, payload.max_relayer_fee
            )));
        }

        let token = match payload.token_address {
            Address::ZERO => shielder_account::Token::Native,
            addr => shielder_account::Token::ERC20(addr),
        };
        let withdraw_circuit = WithdrawCircuit::new(payload.account_id, token);
        let relayer_calldata = withdraw_circuit.get_relayer_calldata(
            payload.withdrawal_value,
            payload.withdraw_address,
            *relay_params.merkle_path,
            payload.chain_id,
            relay_params.relayer_fee,
            payload.pocket_money,
            relay_params.relayer_address,
            payload.protocol_fee,
            payload.memo,
            payload.nullifier_old,
            payload.nullifier_new,
            payload.account_old_balance,
            relay_params.merkle_root,
        );

        Ok(Response::PrepareRelayCalldata {
            calldata: relayer_calldata,
        })
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
            return Err(VsockError::Protocol(format!(
                "Failed to initialize NSM driver (return code) = {}",
                nsm_fd
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
