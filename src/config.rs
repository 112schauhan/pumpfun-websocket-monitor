use anyhow::Result;
use dotenv::dotenv;
use log::info;

mod config;
mod error;

use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logger
    env_logger::init();
    
    info!("Starting pump.fun WebSocket monitor service");
    
    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded successfully");
    info!("WebSocket port: {}", config.websocket_port);
    info!("Solana WS URL: {}", config.solana_ws_url);
    
    // Placeholder for actual service implementation
    info!("Service components will be added in upcoming commits");
    
    // Keep service running briefly to demonstrate
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    info!("Service completed successfully");
    Ok(())
}