use clap::Parser;
mod command_line_args;
mod kms;
mod server;
use log::info;
use shielder_scheduler_common::vsock::VsockError;

use crate::command_line_args::CommandLineArgs;

#[tokio::main]
async fn main() -> Result<(), VsockError> {
    // Parse command line arguments
    let options = CommandLineArgs::parse();

    tracing_subscriber::fmt::init();

    let server = server::Server::new(options).await?;
    info!("Server listening on: {:?}", server.local_addr()?);

    loop {
        let (stream, _) = server.listener().accept().await?;

        let server_clone = server.clone();
        tokio::spawn(async move {
            server_clone.handle_client(stream).await;
        });
    }
}
