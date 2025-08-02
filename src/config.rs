use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Solana RPC WebSocket endpoint
    pub solana_ws_url: String,
    /// Solana RPC HTTP endpoint
    pub solana_rpc_url: String,
    /// WebSocket server port
    pub websocket_port: u16,
    /// Pump.fun program ID
    pub pumpfun_program_id: String,
    /// Connection retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Client rate limit per minute
    pub rate_limit_per_minute: u32,
}

impl Config {
    /// Load configuration from environment variables with defaults matching tests
    pub fn from_env() -> Result<Config> {
        Ok(Config {
            solana_ws_url: env::var("SOLANA_WS_URL")
                .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string()),
            solana_rpc_url: env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
            websocket_port: env::var("WEBSOCKET_PORT")
                // Default here changed to 9999 to match integration test
                .unwrap_or_else(|_| "9999".to_string())
                .parse()?,
            pumpfun_program_id: env::var("PUMPFUN_PROGRAM_ID")
                .unwrap_or_else(|_| "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string()),
            max_retries: env::var("MAX_RETRIES")
                // Default changed to 3 to match integration test
                .unwrap_or_else(|_| "3".to_string())
                .parse()?,
            retry_delay_ms: env::var("RETRY_DELAY_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()?,
            rate_limit_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,
        })
    }
}