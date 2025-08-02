use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::monitor::SolanaMonitor;
use crate::types::{ClientInfo, ClientMessage, ServerMessage, TokenCreatedEvent};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::interval;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

pub struct WebSocketServer {
    config: Config,
    monitor: SolanaMonitor,
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
}

impl WebSocketServer {
    pub async fn new(config: &Config, monitor: SolanaMonitor) -> AppResult<Self> {
        Ok(Self {
            config: config.clone(),
            monitor,
            clients: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    pub async fn start(&self) -> AppResult<()> {
        let addr = format!("0.0.0.0:{}", self.config.websocket_port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("WebSocket server listening on {}", addr);
        
        // Start event broadcaster
        let mut event_receiver = self.monitor.subscribe();
        let clients_broadcast = self.clients.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                Self::broadcast_event(&clients_broadcast, &event).await;
            }
        });
        
        // Accept connections
        while let Ok((stream, addr)) = listener.accept().await {
            let clients = self.clients.clone();
            let config = self.config.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, clients, config).await {
                    error!("Connection error for {}: {}", addr, e);
                }
            });
        }
        
        Ok(())
    }
    
    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
        _config: Config,
    ) -> AppResult<()> {
        info!("New connection from {}", addr);
        
        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        let client_id = Uuid::new_v4().to_string();
        let client_info = ClientInfo {
            id: client_id.clone(),
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            message_count: 0,
            filters: HashMap::new(),
        };
        
        // Add client
        clients.write().insert(client_id.clone(), client_info);
        
        // Send welcome
        let welcome = ServerMessage::Subscribed {
            message: format!("Connected! Client ID: {}", client_id),
        };
        
        if let Ok(welcome_json) = serde_json::to_string(&welcome) {
            let _ = ws_sender.send(Message::Text(welcome_json)).await;
        }
        
        // Handle messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received from {}: {}", addr, text);
                    
                    // Update client activity
                    if let Some(mut client) = clients.write().get_mut(&client_id) {
                        client.last_activity = Utc::now();
                        client.message_count += 1;
                    }
                    
                    // Parse and respond
                    let response = match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Subscribe { filters: _ }) => {
                            ServerMessage::Subscribed {
                                message: "Subscribed to token events!".to_string(),
                            }
                        }
                        Ok(ClientMessage::Unsubscribe) => {
                            ServerMessage::Unsubscribed {
                                message: "Unsubscribed from events".to_string(),
                            }
                        }
                        Ok(ClientMessage::Ping) => ServerMessage::Pong,
                        Err(e) => ServerMessage::Error {
                            message: format!("Invalid message: {}", e),
                        },
                    };
                    
                    if let Ok(response_json) = serde_json::to_string(&response) {
                        if ws_sender.send(Message::Text(response_json)).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} disconnected", addr);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        // Remove client
        clients.write().remove(&client_id);
        Ok(())
    }
    
    async fn broadcast_event(
        clients: &Arc<RwLock<HashMap<String, ClientInfo>>>,
        event: &TokenCreatedEvent,
    ) {
        let client_count = clients.read().len();
        if client_count > 0 {
            info!("Broadcasting event to {} clients: {} ({})", 
                  client_count, event.token.name, event.token.symbol);
        }
    }
    
    pub fn get_client_count(&self) -> usize {
        self.clients.read().len()
    }
}