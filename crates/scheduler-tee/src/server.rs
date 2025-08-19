use std::sync::Arc;

use alloy_primitives::{Address, U256};
#[cfg(not(feature = "without_attestation"))]
use aws_nitro_enclaves_nsm_api::{
    api::Request as NsmRequest,
    api::Response as NsmResponse,
    driver::{nsm_exit, nsm_init, nsm_process_request},
};
use log::{debug, info};
use scheduler_common::{
    protocol::{Payload, Request, Response, SchedulerServer},
    vsock::VsockError,
};
use shielder_setup::consts::{ARITY, TREE_HEIGHT};
use tokio_vsock::{VsockAddr, VsockListener, VsockStream, VMADDR_CID_ANY};

pub struct Server {
    #[cfg(not(feature = "without_attestation"))]
    nsm_fd: i32,

    listener: VsockListener,
}

impl Server {
    pub fn new(port: u16) -> Result<Arc<Self>, VsockError> {
        let address = VsockAddr::new(VMADDR_CID_ANY, port as u32);
        let listener = VsockListener::bind(address)?;

        #[cfg(not(feature = "without_attestation"))]
        let nsm_fd = Self::init_nsm_driver()?;

        #[cfg(feature = "without_attestation")]
        info!("Running server without attestation (TEST BUILD).");

        Ok(Arc::new(Self {
            listener,

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

    pub fn public_key(&self) -> Vec<u8> {
        todo!("Implement public key retrieval logic here");
    }

    pub async fn handle_client(self: Arc<Self>, stream: VsockStream) {
        let result = self.do_handle_client(stream).await;
        debug!("Client disconnected: {result:?}");
    }

    async fn do_handle_client(&self, stream: VsockStream) -> Result<(), VsockError> {
        let mut server: SchedulerServer = stream.into();

        loop {
            server
                .handle_request(|request| match request {
                    Request::Ping => Ok(Response::Pong),
                    Request::TeePublicKey => self.public_key_response(),
                    Request::PrepareRelayCalldata {
                        payload,
                        relayer_address,
                        relayer_fee,
                        merkle_path,
                    } => self.generate_proof_response(
                        payload,
                        relayer_address,
                        relayer_fee,
                        merkle_path,
                    ),
                })
                .await?;
        }
    }

    fn public_key_response(&self) -> Result<Response, VsockError> {
        let public_key = self.public_key();
        let public_key_hex = hex::encode(&public_key);

        #[cfg(not(feature = "without_attestation"))]
        let attestation_document = self.request_attestation_from_nsm_driver(public_key)?;

        #[cfg(feature = "without_attestation")]
        let attestation_document = Vec::new();

        Ok(Response::TeePublicKey {
            public_key: public_key_hex,
            attestation_document,
        })
    }

    fn generate_proof_response(
        &self,
        payload: Vec<u8>,
        _relayer_address: Address,
        _relayer_fee: U256,
        _merkle_path: Box<[[U256; ARITY]; TREE_HEIGHT]>,
    ) -> Result<Response, VsockError> {
        let decrypted_payload = self.decrypt_payload(&payload)?;

        let _deserialized_payload: Payload = serde_json::from_slice(&decrypted_payload)?;

        todo!("Implement proof generation logic here");
    }

    fn decrypt_payload(&self, _payload: &[u8]) -> Result<Vec<u8>, VsockError> {
        todo!("Implement decryption logic here");
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
