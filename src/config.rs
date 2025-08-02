use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub solana_ws_url: String,
    pub solana_rpc_url: String,
    pub websocket_port: u16,
    pub pumpfun_program_id: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub rate_limit_per_minute: u32,
}

impl Config {
    pub fn from_env() -> AppResult<Self> {
        Ok(Config {
            solana_ws_url: env::var("SOLANA_WS_URL")
                .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string()),
            solana_rpc_url: env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
            websocket_port: env::var("WEBSOCKET_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|e| AppError::ConfigError(format!("Invalid port: {}", e)))?,
            pumpfun_program_id: env::var("PUMPFUN_PROGRAM_ID")
                .unwrap_or_else(|_| "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string()),
            max_retries: env::var("MAX_RETRIES")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .map_err(|e| AppError::ConfigError(format!("Invalid max_retries: {}", e)))?,
            retry_delay_ms: env::var("RETRY_DELAY_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .map_err(|e| AppError::ConfigError(format!("Invalid retry_delay_ms: {}", e)))?,
            rate_limit_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .map_err(|e| AppError::ConfigError(format!("Invalid rate_limit: {}", e)))?,
        })
    }
}