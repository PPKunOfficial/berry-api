//! Berry API Server
//! 
//! Main entry point for the Berry API load balancing service

use berry_api::start_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    start_server().await?;
    Ok(())
}
