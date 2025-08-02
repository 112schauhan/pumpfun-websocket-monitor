use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::parser::PumpFunParser;
use crate::types::TokenCreatedEvent;
use log::{debug, error, info, warn};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{interval, sleep};

#[derive(Clone)]
pub struct SolanaMonitor {
    config: Config,
    event_sender: broadcast::Sender<TokenCreatedEvent>,
}

impl SolanaMonitor {
    pub async fn new(config: &Config) -> AppResult<Self> {
        let (event_sender, _) = broadcast::channel(1000);
        
        info!("Solana monitor initialized");
        info!("Target program: {}", config.pumpfun_program_id);
        info!("RPC endpoint: {}", config.solana_rpc_url);
        
        Ok(Self {
            config: config.clone(),
            event_sender,
        })
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<TokenCreatedEvent> {
        self.event_sender.subscribe()
    }
    
    pub async fn start(&self) -> AppResult<()> {
        info!("Starting Solana monitor (mock mode)");
        
        // For now, simulate monitoring with mock data
        self.mock_monitoring_loop().await
    }
    
    async fn mock_monitoring_loop(&self) -> AppResult<()> {
        let mut interval = interval(Duration::from_secs(15));
        let mut counter = 1;
        
        info!("Mock monitoring started - generating events every 15 seconds");
        
        loop {
            interval.tick().await;
            
            // Generate mock transaction
            let mock_tx = format!("pump_mock_transaction_{}", counter);
            
            if let Some(event) = PumpFunParser::parse_mock_transaction(&mock_tx)? {
                info!("Mock token detected: {} ({})", event.token.name, event.token.symbol);
                
                // Broadcast to subscribers
                if let Err(e) = self.event_sender.send(event) {
                    warn!("Failed to broadcast event: {}", e);
                }
            }
            
            counter += 1;
            
            // Stop after 10 iterations for demo
            if counter > 10 {
                info!("Mock monitoring completed (10 events generated)");
                break;
            }
        }
        
        Ok(())
    }
    
    pub fn is_connected(&self) -> bool {
        // For mock implementation, always return true
        true
    }
}