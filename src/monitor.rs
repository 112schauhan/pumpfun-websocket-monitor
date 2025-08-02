use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::parser::PumpFunParser;
use crate::types::{LogsSubscriptionResult, RpcResponse, TokenCreatedEvent};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use parking_lot::RwLock;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Solana blockchain monitor for pump.fun events
#[derive(Clone)]
pub struct SolanaMonitor {
    config: Config,
    event_sender: broadcast::Sender<TokenCreatedEvent>,
    connection_state: Arc<RwLock<ConnectionState>>,
}

#[derive(Debug, Clone)]
struct ConnectionState {
    connected: bool,
    retry_count: u32,
    last_error: Option<String>,
}

impl SolanaMonitor {
    /// Create a new Solana monitor instance
    pub async fn new(config: &Config) -> AppResult<Self> {
        let (event_sender, _) = broadcast::channel(1000);
        
        Ok(Self {
            config: config.clone(),
            event_sender,
            connection_state: Arc::new(RwLock::new(ConnectionState {
                connected: false,
                retry_count: 0,
                last_error: None,
            })),
        })
    }

    /// Get event receiver for new subscribers
    pub fn subscribe(&self) -> broadcast::Receiver<TokenCreatedEvent> {
        self.event_sender.subscribe()
    }

    /// Start monitoring the blockchain
    pub async fn start(&self) -> AppResult<()> {
        info!("Starting Solana monitor for pump.fun events");
        
        loop {
            match self.connect_and_monitor().await {
                Ok(_) => {
                    info!("Monitor connection closed gracefully");
                    break;
                }
                Err(e) => {
                    error!("Monitor connection error: {}", e);
                    
                    // Update connection state
                    {
                        let mut state = self.connection_state.write();
                        state.connected = false;
                        state.retry_count += 1;
                        state.last_error = Some(e.to_string());
                    }

                    // Check if we should retry
                    let retry_count = self.connection_state.read().retry_count;
                    if retry_count >= self.config.max_retries {
                        error!("Max retries exceeded, stopping monitor");
                        return Err(e);
                    }
                    
                    // Wait before retrying
                    let delay = Duration::from_millis(
                        self.config.retry_delay_ms * retry_count as u64
                    );
                    warn!("Retrying connection in {:?} (attempt {})", delay, retry_count + 1);
                    sleep(delay).await;
                }
            }
        }

        Ok(())
    }

    /// Connect to Solana WebSocket and start monitoring
    async fn connect_and_monitor(&self) -> AppResult<()> {
        info!("Connecting to Solana WebSocket: {}", self.config.solana_ws_url);
        
        let (ws_stream, _) = connect_async(&self.config.solana_ws_url)
            .await
            .map_err(|e| AppError::ConnectionError(format!("Failed to connect: {}", e)))?;

        let (write, mut read) = ws_stream.split();

        // Wrap write in an Arc<Mutex<_>> to share safely across tasks
        let write = Arc::new(Mutex::new(write));
        let write_clone = write.clone();

        // Update connection state
        {
            let mut state = self.connection_state.write();
            state.connected = true;
            state.retry_count = 0;
            state.last_error = None;
        }

        info!("Connected to Solana WebSocket successfully");

        // Subscribe to logs for the pump.fun program
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "logsSubscribe",
            "params": [
                {
                    "mentions": [self.config.pumpfun_program_id]
                },
                {
                    "commitment": "confirmed"
                }
            ]
        });

        write.lock().await
            .send(Message::Text(subscribe_request.to_string()))
            .await
            .map_err(AppError::WebSocketError)?;

        info!("Subscribed to pump.fun program logs");

        // Start heartbeat task
        let heartbeat_task = tokio::spawn(async move {
            let mut heartbeat_interval = interval(Duration::from_secs(30));
            loop {
                heartbeat_interval.tick().await;
                let mut locked_write = write_clone.lock().await;
                if let Err(e) = locked_write.send(Message::Ping(vec![])).await {
                    error!("Heartbeat failed: {}", e);
                    break;
                }
            }
        });

        // Process incoming messages
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.process_message(&text).await {
                        warn!("Error processing message: {}", e);
                    }
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        // Clean up heartbeat task
        heartbeat_task.abort();

        Ok(())
    }

    /// Process incoming WebSocket message
    async fn process_message(&self, text: &str) -> AppResult<()> {
        debug!("Received message: {}", text);

        // Parse the RPC response
        let response: RpcResponse<serde_json::Value> = serde_json::from_str(text)
            .map_err(|e| AppError::ParseError(format!("Failed to parse RPC response: {}", e)))?;

        // Handle subscription notifications
        if let Some(method) = response.result.as_ref().and_then(|r| r.get("method")) {
            if method == "logsNotification" {
                if let Some(params) = response.result.as_ref().and_then(|r| r.get("params")) {
                    if let Some(result) = params.get("result") {
                        let logs_result: LogsSubscriptionResult = serde_json::from_value(result.clone())
                            .map_err(|e| AppError::ParseError(format!("Failed to parse logs result: {}", e)))?;

                        if let Some(event) = PumpFunParser::parse_token_creation(&logs_result.value)? {
                            // Validate the event
                            PumpFunParser::validate_event(&event)?;
                            
                            info!("Token creation detected: {} ({})", event.token.name, event.token.symbol);
                            
                            // Broadcast to all subscribers
                            if let Err(e) = self.event_sender.send(event) {
                                warn!("Failed to broadcast event: {}", e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get current connection status
    pub fn is_connected(&self) -> bool {
        self.connection_state.read().connected
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionState {
        self.connection_state.read().clone()
    }
}
