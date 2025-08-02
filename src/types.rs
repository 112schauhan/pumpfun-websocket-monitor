use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token creation event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCreatedEvent {
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_signature: String,
    pub token: TokenInfo,
    pub pump_data: PumpData,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint_address: String,
    pub name: String,
    pub symbol: String,
    pub creator: String,
    pub supply: u64,
    pub decimals: u8,
}

/// Pump.fun specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpData {
    pub bonding_curve: String,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
}

/// Solana RPC subscription response
#[derive(Debug, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

/// RPC error structure
#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Account subscription result
#[derive(Debug, Deserialize)]
pub struct AccountSubscriptionResult {
    pub context: RpcContext,
    pub value: AccountInfo,
}

/// RPC context
#[derive(Debug, Deserialize)]
pub struct RpcContext {
    pub slot: u64,
}

/// Account information
#[derive(Debug, Deserialize)]
pub struct AccountInfo {
    pub data: Vec<String>,
    pub executable: bool,
    pub lamports: u64,
    pub owner: String,
    pub rent_epoch: u64,
}

/// Log subscription result
#[derive(Debug, Deserialize)]
pub struct LogsSubscriptionResult {
    pub context: RpcContext,
    pub value: LogsInfo,
}

/// Logs information
#[derive(Debug, Deserialize)]
pub struct LogsInfo {
    pub signature: String,
    pub err: Option<serde_json::Value>,
    pub logs: Vec<String>,
}

/// WebSocket client connection info
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: String,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub message_count: u64,
    pub filters: HashMap<String, serde_json::Value>,
}

/// Client message types
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { filters: Option<HashMap<String, serde_json::Value>> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
    #[serde(rename = "ping")]
    Ping,
}

/// Server message types
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "token_created")]
    TokenCreated(TokenCreatedEvent),
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "subscribed")]
    Subscribed { message: String },
    #[serde(rename = "unsubscribed")]
    Unsubscribed { message: String },
}