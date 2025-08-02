use anyhow::Result;
use dotenv::dotenv;
use env_logger::Env;
use log::info;
use std::env;
use tokio::signal;

mod config;
mod error;
mod monitor;
mod parser;
mod server;
mod types;

use config::Config;
use monitor::SolanaMonitor;
use server::WebSocketServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    info!("Starting pump.fun WebSocket monitor service");
    
    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded successfully");
    
    // Create and start the monitor
    let monitor = SolanaMonitor::new(&config).await?;
    info!("Solana monitor initialized");
    
    // Create and start the WebSocket server
    let server = WebSocketServer::new(&config, monitor.clone()).await?;
    info!("WebSocket server initialized on port {}", config.websocket_port);
    
    // Start both services concurrently
    let monitor_handle = tokio::spawn(async move {
        if let Err(e) = monitor.start().await {
            log::error!("Monitor error: {}", e);
        }
    });
    
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            log::error!("Server error: {}", e);
        }
    });
    
    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        _ = monitor_handle => {
            log::error!("Monitor service stopped unexpectedly");
        }
        _ = server_handle => {
            log::error!("WebSocket server stopped unexpectedly");
        }
    }
    
    info!("Shutting down gracefully");
    Ok(())
}