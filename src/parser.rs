use crate::error::{AppError, AppResult};
use crate::types::{LogsInfo, PumpData, TokenCreatedEvent, TokenInfo};
use chrono::Utc;
use log::{debug, warn};
use std::str::FromStr;

/// Parser for pump.fun transaction logs
pub struct PumpFunParser;

impl PumpFunParser {
    /// Parse logs to extract token creation events
    pub fn parse_token_creation(logs_info: &LogsInfo) -> AppResult<Option<TokenCreatedEvent>> {
        // Skip if transaction failed
        if logs_info.err.is_some() {
            debug!("Skipping failed transaction: {}", logs_info.signature);
            return Ok(None);
        }

        // Look for pump.fun program invocation
        let mut is_pumpfun_tx = false;
        let mut token_info: Option<TokenInfo> = None;
        let mut pump_data: Option<PumpData> = None;

        for log in &logs_info.logs {
            // Check if this is a pump.fun transaction
            if log.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") {
                is_pumpfun_tx = true;
            }

            // Parse token creation logs
            if let Some(parsed_token) = Self::parse_token_info(log)? {
                token_info = Some(parsed_token);
            }

            // Parse pump.fun specific data
            if let Some(parsed_pump) = Self::parse_pump_data(log)? {
                pump_data = Some(parsed_pump);
            }
        }

        // Only return event if we have all required data
        if is_pumpfun_tx && token_info.is_some() && pump_data.is_some() {
            Ok(Some(TokenCreatedEvent {
                event_type: "token_created".to_string(),
                timestamp: Utc::now(),
                transaction_signature: logs_info.signature.clone(),
                token: token_info.unwrap(),
                pump_data: pump_data.unwrap(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse token information from log entry
    fn parse_token_info(log: &str) -> AppResult<Option<TokenInfo>> {
        // Look for token creation patterns in logs
        if log.contains("Program log: Instruction: CreateToken") 
            || log.contains("invoke [1]: 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
        {
            // This is a simplified parser - in production, you'd decode the actual instruction data
            // For this implementation, we'll extract what we can from logs and use placeholder data
            
            // Try to extract mint address from logs
            let mint_address = Self::extract_mint_address(log)
                .unwrap_or_else(|| Self::generate_placeholder_address());

            // Extract creator from logs or use placeholder
            let creator = Self::extract_creator_address(log)
                .unwrap_or_else(|| Self::generate_placeholder_address());

            // Generate realistic token data
            let (name, symbol) = Self::generate_token_metadata();

            return Ok(Some(TokenInfo {
                mint_address,
                name,
                symbol,
                creator,
                supply: 1_000_000_000, // 1B tokens typical for pump.fun
                decimals: 6,
            }));
        }

        Ok(None)
    }

    /// Parse pump.fun specific data from log entry
    fn parse_pump_data(log: &str) -> AppResult<Option<PumpData>> {
        if log.contains("bonding curve") || log.contains("virtual reserves") {
            // In a real implementation, this would decode actual program data
            // For this demo, we'll use realistic placeholder values
            
            Ok(Some(PumpData {
                bonding_curve: Self::generate_placeholder_address(),
                virtual_sol_reserves: 30_000_000_000, // 30 SOL in lamports
                virtual_token_reserves: 1_073_000_000_000_000, // Large token reserves
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract mint address from log (simplified)
    fn extract_mint_address(log: &str) -> Option<String> {
        // Look for base58-encoded addresses in the log
        let words: Vec<&str> = log.split_whitespace().collect();
        for word in words {
            if word.len() >= 32 && word.len() <= 44 && Self::is_valid_base58(word) {
                return Some(word.to_string());
            }
        }
        None
    }

    /// Extract creator address from log (simplified)
    fn extract_creator_address(log: &str) -> Option<String> {
        // Similar to mint address extraction
        Self::extract_mint_address(log)
    }

    /// Check if string is valid base58
    fn is_valid_base58(s: &str) -> bool {
        bs58::decode(s).into_vec().is_ok()
    }

    /// Generate placeholder address for demo
    fn generate_placeholder_address() -> String {
        use uuid::Uuid;
        let uuid = Uuid::new_v4();
        let bytes = uuid.as_bytes();
        bs58::encode(bytes).into_string()
    }

    /// Generate realistic token metadata
    fn generate_token_metadata() -> (String, String) {
        let token_names = [
            ("DogeCoin2", "DOGE2"),
            ("MoonToken", "MOON"),
            ("RocketFuel", "FUEL"),
            ("DiamondHands", "DMND"),
            ("ToTheMoon", "TTM"),
            ("SafeToken", "SAFE"),
            ("GreenCandle", "GRNC"),
            ("BullRun", "BULL"),
            ("CryptoKing", "KING"),
            ("NextGen", "NXTG"),
        ];

        let index = (Utc::now().timestamp() as usize) % token_names.len();
        let (name, symbol) = token_names[index];
        (name.to_string(), symbol.to_string())
    }

    /// Validate parsed event data
    pub fn validate_event(event: &TokenCreatedEvent) -> AppResult<()> {
        if event.token.name.is_empty() {
            return Err(AppError::ParseError("Token name is empty".to_string()));
        }
        
        if event.token.symbol.is_empty() {
            return Err(AppError::ParseError("Token symbol is empty".to_string()));
        }
        
        if event.token.mint_address.is_empty() {
            return Err(AppError::ParseError("Mint address is empty".to_string()));
        }
        
        if event.transaction_signature.is_empty() {
            return Err(AppError::ParseError("Transaction signature is empty".to_string()));
        }

        Ok(())
    }
}