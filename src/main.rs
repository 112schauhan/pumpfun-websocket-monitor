use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use serde_json;

mod config;
mod error;
mod types;
mod parser;

use dotenv::dotenv;
use config::Config;
use error::AppResult;
use parser::PumpFunParser;
use types::{ClientMessage, ServerMessage, TokenCreatedEvent, TokenInfo, PumpData};

#[tokio::main]
async fn main() -> AppResult<()> {

    dotenv().ok();
    // Initialize logger
    env_logger::init();
    
    info!("Starting pump.fun WebSocket monitor service");
    info!("This version demonstrates our data types and message handling");
    
    // Demonstrate our types work correctly
    demonstrate_types();
    let config: Config = Config::from_env()?;
    info!("Configuration loaded successfully");
    info!("WebSocket port: {}", config.websocket_port);
    
    // Start basic WebSocket server
    let addr = format!("127.0.0.1:{}", config.websocket_port);
    let listener = TcpListener::bind(&addr).await?;
    info!("WebSocket server listening on: {}", addr);
    info!("Connect with: wscat -c ws://127.0.0.1:8080");
    info!("Try: {{\"type\": \"subscribe\"}} or {{\"type\": \"ping\"}}");
    
    while let Ok((stream, addr)) = listener.accept().await {
        info!("New connection from: {}", addr);
        tokio::spawn(handle_connection(stream, addr));
    }
    
    Ok(())
}

fn demonstrate_types() {
    info!("=== Demonstrating Data Types ===");
    
    // Create sample token data
    let token_info = TokenInfo {
        mint_address: "ABC123xyz789".to_string(),
        name: "SampleToken".to_string(),
        symbol: "SMPL".to_string(),
        creator: "Creator123xyz".to_string(),
        supply: 1_000_000_000,
        decimals: 6,
    };
    
    let pump_data = PumpData {
        bonding_curve: "BondingCurve123".to_string(),
        virtual_sol_reserves: 30_000_000_000,
        virtual_token_reserves: 1_073_000_000_000_000,
    };
    
    let token_event = TokenCreatedEvent {
        event_type: "token_created".to_string(),
        timestamp: Utc::now(),
        transaction_signature: "TxSignature123xyz".to_string(),
        token: token_info,
        pump_data,
    };
    
    // Demonstrate serialization
    match serde_json::to_string_pretty(&token_event) {
        Ok(json) => {
            info!("Sample TokenCreatedEvent JSON:");
            println!("{}", json);
        }
        Err(e) => error!("Failed to serialize token event: {}", e),
    }
    
    info!("=== Types demonstration complete ===");
}

async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    match accept_async(stream).await {
        Ok(ws_stream) => {
            info!("WebSocket connection established with {}", addr);
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();
            
            // Send welcome message using our ServerMessage enum
            let welcome = ServerMessage::Subscribed {
                message: "Connected! You can send: subscribe, unsubscribe, or ping".to_string(),
            };
            
            if let Ok(welcome_json) = serde_json::to_string(&welcome) {
                if let Err(e) = ws_sender.send(Message::Text(welcome_json)).await {
                    error!("Failed to send welcome message: {}", e);
                    return;
                }
            }
            
            // Handle incoming messages
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        info!("Received from {}: {}", addr, text);
                        
                        // Try to parse using our ClientMessage enum
                        let response = match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Subscribe { filters: _ }) => {
                                info!("Client {} subscribed", addr);
                                ServerMessage::Subscribed {
                                    message: "Successfully subscribed to token events!".to_string(),
                                }
                            }
                            Ok(ClientMessage::Unsubscribe) => {
                                info!("Client {} unsubscribed", addr);
                                ServerMessage::Unsubscribed {
                                    message: "Successfully unsubscribed".to_string(),
                                }
                            }
                            Ok(ClientMessage::Ping) => {
                                info!("Ping from {}", addr);
                                ServerMessage::Pong
                            }
                            Err(e) => {
                                warn!("Invalid message from {}: {}", addr, e);
                                ServerMessage::Error {
                                    message: format!("Invalid message format: {}", e),
                                }
                            }
                        };
                        
                        // Send response using our types
                        if let Ok(response_json) = serde_json::to_string(&response) {
                            if let Err(e) = ws_sender.send(Message::Text(response_json)).await {
                                error!("Failed to send response: {}", e);
                                break;
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Client {} disconnected", addr);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error with {}: {}", addr, e);
                        break;
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            error!("Failed to accept WebSocket connection from {}: {}", addr, e);
        }
    }
}

fn demonstrate_parser() {
    info!("=== Demonstrating Parser ===");
    
    let mock_transactions = [
        "pump_token_creation_123",
        "regular_transaction_456", 
        "pump_new_token_789"
    ];
    
    for tx in &mock_transactions {
        match PumpFunParser::parse_mock_transaction(tx) {
            Ok(Some(event)) => {
                info!("Parsed token creation: {} ({})", event.token.name, event.token.symbol);
            }
            Ok(None) => {
                info!("No token creation in transaction: {}", tx);
            }
            Err(e) => {
                warn!("Parse error for {}: {}", tx, e);
            }
        }
    }
    
    info!("=== Parser demonstration complete ===");
}