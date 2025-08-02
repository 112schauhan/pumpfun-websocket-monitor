use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(#[from] tungstenite::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type AppResult<T> = Result<T, AppError>;