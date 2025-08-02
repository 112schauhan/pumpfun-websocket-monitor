use anyhow::Result;
use dotenv::dotenv;
use env_logger::Env;
use log::info;
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
    // Load environment variables from .env file
    dotenv().ok();
    // Initialize logger with default level 'info' or from RUST_LOG env var
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Starting pump.fun WebSocket monitor service");

    // Load configuration from environment variables
    let config = Config::from_env()?;
    info!("Configuration loaded successfully");

    // Print startup information
    print_startup_info(&config);

    // Create and initialize the Solana monitor
    let monitor = SolanaMonitor::new(&config).await?;
    info!("Solana monitor initialized");

    // Create and initialize the WebSocket server
    let server = WebSocketServer::new(&config, monitor.clone()).await?;
    info!("WebSocket server initialized on port {}", config.websocket_port);

    // Start both services concurrently
    let monitor_handle = tokio::spawn({
        let monitor = monitor.clone();
        async move {
            if let Err(e) = monitor.start().await {
                log::error!("Monitor service error: {}", e);
            }
        }
    });

    let server_handle = tokio::spawn({
        async move {
            if let Err(e) = server.start().await {
                log::error!("WebSocket server error: {}", e);
            }
        }
    });

    // Wait for shutdown signal or service failure
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal (Ctrl+C)");
        }
        result = monitor_handle => {
            match result {
                Ok(_) => info!("Monitor service completed"),
                Err(e) => log::error!("Monitor service panicked: {}", e),
            }
        }
        result = server_handle => {
            match result {
                Ok(_) => info!("WebSocket server completed"),
                Err(e) => log::error!("WebSocket server panicked: {}", e),
            }
        }
    }

    info!("Shutting down gracefully...");
    // Give services a moment to clean up
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    info!("Shutdown complete");

    Ok(())
}

/// Print startup information and instructions for users
fn print_startup_info(config: &Config) {
    info!("🚀 Pump.fun WebSocket Monitor Started Successfully!");
    info!("📡 WebSocket Server: ws://localhost:{}", config.websocket_port);
    info!("🔗 Solana Network: {}", config.solana_ws_url);
    info!("📊 Program ID: {}", config.pumpfun_program_id);
    info!("⚙️ Max Retries: {}", config.max_retries);
    info!("🕐 Retry Delay: {}ms", config.retry_delay_ms);
    info!("🚦 Rate Limit: {} messages/minute", config.rate_limit_per_minute);

    println!("\n📋 How to connect:");
    println!(" JavaScript: wscat -c ws://localhost:{}", config.websocket_port);
    println!(" Python: python examples/test_client.py");
    println!(" Node.js: node examples/test_client.js");
    println!("\n📨 Sample messages:");
    println!(" Subscribe: {{\"type\": \"subscribe\"}}");
    println!(" Unsubscribe: {{\"type\": \"unsubscribe\"}}");
    println!(" Ping: {{\"type\": \"ping\"}}");
    println!("\n🛑 To stop: Press Ctrl+C");
    println!("{}", "─".repeat(60));
}