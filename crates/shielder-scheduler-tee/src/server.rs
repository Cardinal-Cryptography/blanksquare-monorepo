use std::sync::Arc;

use alloy_primitives::{Address, U256};
#[cfg(not(feature = "without_attestation"))]
use aws_nitro_enclaves_nsm_api::{
    api::Request as NsmRequest,
    api::Response as NsmResponse,
    driver::{nsm_exit, nsm_init, nsm_process_request},
};
use log::{debug, info};
use shielder_scheduler_common::{
    protocol::{Payload, Request, Response, TEEServer},
    vsock::VsockError,
};
use shielder_setup::consts::{ARITY, TREE_HEIGHT};

use tokio_vsock::{VsockAddr, VsockListener, VsockStream, VMADDR_CID_ANY};


use crate::withdraw::WithdrawCircuit;

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
        // TODO: Implement public key retrieval logic
        vec![0; 32] // Placeholder for the public key
    }

    pub async fn handle_client(self: Arc<Self>, stream: VsockStream) {
        let result = self.do_handle_client(stream).await;
        debug!("Client disconnected: {result:?}");
    }

    async fn do_handle_client(&self, stream: VsockStream) -> Result<(), VsockError> {
        let mut server: TEEServer = stream.into();

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
                        merkle_root,
                    } => self.prepare_relay_calldata_response(
                        payload,
                        relayer_address,
                        relayer_fee,
                        merkle_path,
                        merkle_root,
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

    fn prepare_relay_calldata_response(
        &self,
        payload: Vec<u8>,
        relayer_address: Address,
        relayer_fee: U256,
        merkle_path: Box<[[U256; ARITY]; TREE_HEIGHT]>,
        merkle_root: U256,
    ) -> Result<Response, VsockError> {
        let decrypted_payload = self.decrypt_payload(&payload)?;

        let deserialized_payload : Payload = serde_json::from_slice(&decrypted_payload).map_err(|e| {
            VsockError::Protocol(format!("Failed to deserialize payload: {}", e))
        })?;

        let token = match deserialized_payload.token_address {
            Address::ZERO => shielder_account::Token::Native,
            addr => shielder_account::Token::ERC20(addr),
        };
        let withdraw_circuit = WithdrawCircuit::new(deserialized_payload.account_id, 
            token);
        let relayer_calldata = withdraw_circuit.get_relayer_calldata(
            deserialized_payload.withdrawal_value,
            deserialized_payload.withdraw_address,
            *merkle_path,
            deserialized_payload.chain_id,
            relayer_fee,
            deserialized_payload.pocket_money,
            relayer_address,
            deserialized_payload.protocol_fee,
            deserialized_payload.memo,
            deserialized_payload.nullifier_old,
            deserialized_payload.nullifier_new,
            deserialized_payload.account_old_balance,
            merkle_root,
        );

        Ok(Response::PrepareRelayCalldata { calldata: relayer_calldata })
    }

    fn decrypt_payload(&self, _payload: &[u8]) -> Result<Vec<u8>, VsockError> {
        // TODO: Implement decryption logic here
        Ok(_payload.to_vec()) // Placeholder for decrypted payload
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
