use std::time::Duration;
use tokio::time::timeout;
use pumpfun_websocket_monitor::{Config, SolanaMonitor};

#[tokio::test]
async fn test_config_from_env() {
    // Test configuration loading
    std::env::set_var("WEBSOCKET_PORT", "9999");
    std::env::set_var("MAX_RETRIES", "3");
    
    let config = Config::from_env().unwrap();
    assert_eq!(config.websocket_port, 9999);
    assert_eq!(config.max_retries, 3);
    
    // Clean up
    std::env::remove_var("WEBSOCKET_PORT");
    std::env::remove_var("MAX_RETRIES");
}

#[tokio::test]
async fn test_monitor_creation() {
    let config = Config::from_env().unwrap();
    let monitor = SolanaMonitor::new(&config).await;
    
    assert!(monitor.is_ok());
    
    let monitor = monitor.unwrap();
    assert!(!monitor.is_connected()); // Should not be connected initially
}

#[tokio::test]
async fn test_config_defaults() {
    // Test that defaults work when no env vars are set
    let config = Config::from_env().unwrap();
    
    assert_eq!(config.websocket_port, 9999);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.pumpfun_program_id, "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
}

#[tokio::test]
async fn test_monitor_subscription() {
    let config = Config::from_env().unwrap();
    let monitor = SolanaMonitor::new(&config).await.unwrap();
    
    // Test that we can create event receivers
    let _receiver1 = monitor.subscribe();
    let _receiver2 = monitor.subscribe();
    
    // Multiple receivers should work
    assert!(!monitor.is_connected());
}

// Basic smoke test - just ensure the server can be created
#[tokio::test]
async fn test_server_creation_smoke_test() {
    // Use a test port to avoid conflicts
    std::env::set_var("WEBSOCKET_PORT", "8081");
    
    let config = Config::from_env().unwrap();
    let monitor = SolanaMonitor::new(&config).await.unwrap();
    
    // Just test creation, not actual startup
    let server_result = pumpfun_websocket_monitor::WebSocketServer::new(&config, monitor).await;
    assert!(server_result.is_ok());
    
    // Clean up
    std::env::remove_var("WEBSOCKET_PORT");
}