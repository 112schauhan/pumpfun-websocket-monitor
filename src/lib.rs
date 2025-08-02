pub mod config;
pub mod error;
pub mod monitor;
pub mod parser;
pub mod server;
pub mod types;

// Re-export main types for convenience
pub use config::Config;
pub use error::{AppError, AppResult};
pub use monitor::SolanaMonitor;
pub use parser::PumpFunParser;
pub use server::WebSocketServer;
pub use types::{
    TokenCreatedEvent, TokenInfo, PumpData, ClientInfo,
    ClientMessage, ServerMessage
};

/// Current version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Pump.fun program ID on Solana
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
/// Default WebSocket port
pub const DEFAULT_WEBSOCKET_PORT: u16 = 8081;
/// Default Solana mainnet WebSocket URL
pub const DEFAULT_SOLANA_WS_URL: &str = "wss://api.mainnet-beta.solana.com";
/// Default Solana mainnet RPC URL
pub const DEFAULT_SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
