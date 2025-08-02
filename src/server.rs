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
use tokio::time::{interval, Instant};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

/// WebSocket server for streaming pump.fun events to clients
pub struct WebSocketServer {
    config: Config,
    monitor: SolanaMonitor,
    clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
}

/// Client connection wrapper
struct ClientConnection {
    info: ClientInfo,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
    last_message_time: Instant,
    message_count_in_window: u32,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub async fn new(config: &Config, monitor: SolanaMonitor) -> AppResult<Self> {
        Ok(Self {
            config: config.clone(),
            monitor,
            clients: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start the WebSocket server
    pub async fn start(&self) -> AppResult<()> {
        let addr = format!("0.0.0.0:{}", self.config.websocket_port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("WebSocket server listening on {}", addr);

        // Start cleanup task for disconnected clients
        let clients_cleanup = self.clients.clone();
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(60));
            loop {
                cleanup_interval.tick().await;
                Self::cleanup_disconnected_clients(&clients_cleanup);
            }
        });

        // Start event broadcaster task
        let mut event_receiver = self.monitor.subscribe();
        let clients_broadcast = self.clients.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                Self::broadcast_event(&clients_broadcast, &event).await;
            }
        });

        // Accept incoming connections
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

    /// Handle a new WebSocket connection
    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
        config: Config,
    ) -> AppResult<()> {
        info!("New connection from {}", addr);

        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Create client info
        let client_id = Uuid::new_v4().to_string();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let client_info = ClientInfo {
            id: client_id.clone(),
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            message_count: 0,
            filters: HashMap::new(),
        };

        let client_connection = ClientConnection {
            info: client_info,
            sender: tx,
            last_message_time: Instant::now(),
            message_count_in_window: 0,
        };

        // Add client to the map
        clients.write().insert(client_id.clone(), client_connection);

        info!("Client {} connected from {}", client_id, addr);

        // Send welcome message
        let welcome_msg = ServerMessage::Subscribed {
            message: "Connected to pump.fun monitor".to_string(),
        };
        if let Ok(welcome_json) = serde_json::to_string(&welcome_msg) {
            if let Some(client) = clients.read().get(&client_id) {
                let _ = client.sender.send(Message::Text(welcome_json));
            }
        }

        // Spawn task to send messages to client
        let client_id_send = client_id.clone();
        let clients_send = clients.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if ws_sender.send(message).await.is_err() {
                    break;
                }
            }
            // Remove client when sender task ends
            clients_send.write().remove(&client_id_send);
        });

        // Handle incoming messages from client
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = Self::handle_client_message(&clients, &client_id, &text, &config).await {
                        warn!("Error handling client message: {}", e);
                    }
                }
                Ok(Message::Ping(data)) => {
                    if let Some(client) = clients.read().get(&client_id) {
                        let _ = client.sender.send(Message::Pong(data));
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} disconnected", client_id);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error for client {}: {}", client_id, e);
                    break;
                }
                _ => {}
            }
        }

        // Remove client on disconnect
        clients.write().remove(&client_id);
        info!("Client {} removed", client_id);

        Ok(())
    }

    /// Handle incoming client message
    async fn handle_client_message(
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        client_id: &str,
        text: &str,
        config: &Config,
    ) -> AppResult<()> {
        debug!("Received message from client {}: {}", client_id, text);

        // Rate limiting check
        {
            let mut clients_guard = clients.write();
            if let Some(client) = clients_guard.get_mut(client_id) {
                let now = Instant::now();
                
                // Reset counter every minute
                if now.duration_since(client.last_message_time).as_secs() >= 60 {
                    client.message_count_in_window = 0;
                    client.last_message_time = now;
                }
                
                client.message_count_in_window += 1;
                
                if client.message_count_in_window > config.rate_limit_per_minute {
                    let error_msg = ServerMessage::Error {
                        message: "Rate limit exceeded".to_string(),
                    };
                    if let Ok(error_json) = serde_json::to_string(&error_msg) {
                        let _ = client.sender.send(Message::Text(error_json));
                    }
                    return Err(AppError::RateLimitExceeded);
                }
                
                // Update last activity
                client.info.last_activity = Utc::now();
                client.info.message_count += 1;
            }
        }

        // Parse client message
        let client_message: ClientMessage = serde_json::from_str(text)
            .map_err(|e| AppError::ParseError(format!("Invalid client message: {}", e)))?;

        let response = match client_message {
            ClientMessage::Subscribe { filters } => {
                // Update client filters
                if let Some(filters) = filters {
                    if let Some(client) = clients.write().get_mut(client_id) {
                        client.info.filters = filters;
                    }
                }
                
                ServerMessage::Subscribed {
                    message: "Subscribed to token creation events".to_string(),
                }
            }
            ClientMessage::Unsubscribe => {
                // Clear client filters
                if let Some(client) = clients.write().get_mut(client_id) {
                    client.info.filters.clear();
                }
                
                ServerMessage::Unsubscribed {
                    message: "Unsubscribed from events".to_string(),
                }
            }
            ClientMessage::Ping => ServerMessage::Pong,
        };

        // Send response
        if let Some(client) = clients.read().get(client_id) {
            if let Ok(response_json) = serde_json::to_string(&response) {
                let _ = client.sender.send(Message::Text(response_json));
            }
        }

        Ok(())
    }

    /// Broadcast event to all connected clients
    async fn broadcast_event(
        clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
        event: &TokenCreatedEvent,
    ) {
        let clients_snapshot: Vec<(String, tokio::sync::mpsc::UnboundedSender<Message>)> = {
            clients
                .read()
                .iter()
                .map(|(id, conn)| (id.clone(), conn.sender.clone()))
                .collect()
        };

        if !clients_snapshot.is_empty() {
            let event_message = ServerMessage::TokenCreated(event.clone());
            
            if let Ok(event_json) = serde_json::to_string(&event_message) {
                let message = Message::Text(event_json);
                
                for (client_id, sender) in clients_snapshot {
                    if sender.send(message.clone()).is_err() {
                        debug!("Failed to send to client {}, will be cleaned up", client_id);
                    }
                }
                
                info!("Broadcasted event to {} clients", clients.read().len());
            }
        }
    }

    /// Clean up disconnected clients
    fn cleanup_disconnected_clients(clients: &Arc<RwLock<HashMap<String, ClientConnection>>>) {
        let mut to_remove = Vec::new();
        
        {
            let clients_guard = clients.read();
            for (client_id, client) in clients_guard.iter() {
                // Remove clients inactive for more than 5 minutes
                if Utc::now().signed_duration_since(client.info.last_activity).num_minutes() > 5 {
                    to_remove.push(client_id.clone());
                }
            }
        }
        
        if !to_remove.is_empty() {
            let mut clients_guard = clients.write();
            for client_id in to_remove {
                clients_guard.remove(&client_id);
                debug!("Cleaned up inactive client: {}", client_id);
            }
        }
    }

    /// Get current client count
    pub fn get_client_count(&self) -> usize {
        self.clients.read().len()
    }

    /// Get client statistics
    pub fn get_client_stats(&self) -> Vec<ClientInfo> {
        self.clients
            .read()
            .values()
            .map(|conn| conn.info.clone())
            .collect()
    }
}

/// Broadcast event to all connected clients with per-client filter support
async fn broadcast_event(
    clients: &Arc<RwLock<HashMap<String, ClientConnection>>>,
    event: &TokenCreatedEvent,
) {
    // Prepare snapshot with client filters
    let clients_snapshot: Vec<(String, tokio::sync::mpsc::UnboundedSender<Message>, HashMap<String, serde_json::Value>)> = {
        let guard = clients.read();
        guard.iter()
            .map(|(id, conn)| (id.clone(), conn.sender.clone(), conn.info.filters.clone()))
            .collect()
    };

    if !clients_snapshot.is_empty() {
        let event_message = ServerMessage::TokenCreated(event.clone());
        if let Ok(event_json) = serde_json::to_string(&event_message) {
            let message = Message::Text(event_json);

            for (client_id, sender, filters) in clients_snapshot {
                // Example filter: min_supply
                let mut filtered_out = false;
                if let Some(min_supply) = filters
                    .get("min_supply")
                    .and_then(|v| v.as_u64())
                {
                    if event.token.supply < min_supply {
                        filtered_out = true;
                    }
                }
                // Implement additional filters as needed...

                if filtered_out {
                    continue;
                }

                if sender.send(message.clone()).is_err() {
                    // Send failed - client will be cleaned up in cleanup task
                    debug!("Failed to send to client {}, will be cleaned up", client_id);
                }
            }
        }
    }

    info!("Broadcasted event to {} clients", clients.read().len());
}