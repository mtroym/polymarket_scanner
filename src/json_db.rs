use crate::error::{Result, ScannerError};
use crate::storage::Storage;
use crate::types::{EventType, Market, MarketEvent};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MarketData {
    markets: HashMap<String, Market>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EventData {
    events: Vec<MarketEvent>,
}

pub struct JsonDatabase {
    base_path: PathBuf,
    markets: RwLock<HashMap<String, Market>>,
    events: RwLock<Vec<MarketEvent>>,
    price_history: RwLock<HashMap<String, Vec<(String, String, DateTime<Utc>)>>>,
}

impl JsonDatabase {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
            markets: RwLock::new(HashMap::new()),
            events: RwLock::new(Vec::new()),
            price_history: RwLock::new(HashMap::new()),
        }
    }

    async fn save_to_file<T: Serialize>(&self, filename: &str, data: &T) -> Result<()> {
        let file_path = self.base_path.join(filename);
        let temp_path = self.base_path.join(format!("{}.tmp", filename));

        let json = serde_json::to_string_pretty(data).map_err(|e| ScannerError::JsonError(e))?;

        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Failed to create temp file: {}", e)))?;

        file.write_all(json.as_bytes()).await.map_err(|e| {
            ScannerError::ConfigError(format!("Failed to write to temp file: {}", e))
        })?;

        file.flush()
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Failed to flush temp file: {}", e)))?;

        fs::rename(&temp_path, &file_path)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Failed to rename temp file: {}", e)))?;

        Ok(())
    }

    async fn load_from_file<T: for<'a> Deserialize<'a>>(
        &self,
        filename: &str,
    ) -> Result<Option<T>> {
        let file_path = self.base_path.join(filename);

        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Failed to read file: {}", e)))?;

        let data = serde_json::from_str(&content).map_err(|e| ScannerError::JsonError(e))?;

        Ok(Some(data))
    }
}

#[async_trait]
impl Storage for JsonDatabase {
    async fn init(&self) -> Result<()> {
        if !self.base_path.exists() {
            fs::create_dir_all(&self.base_path).await.map_err(|e| {
                ScannerError::ConfigError(format!("Failed to create data directory: {}", e))
            })?;
        }

        // Load markets
        if let Some(data) = self.load_from_file::<MarketData>("markets.json").await? {
            let mut markets = self.markets.write().await;
            *markets = data.markets;
            info!("Loaded {} markets from disk", markets.len());
        }

        // Load events
        if let Some(data) = self.load_from_file::<EventData>("events.json").await? {
            let mut events = self.events.write().await;
            *events = data.events;
            info!("Loaded {} events from disk", events.len());
        }

        // Load history (simplified: just keeping in memory for now or implement separate file per market later)
        // For this MVP, we'll skip loading history from disk to keep it simple,
        // or we could implement a simple history.json if needed.

        Ok(())
    }

    async fn save_market(&self, market: &Market) -> Result<()> {
        {
            let mut markets = self.markets.write().await;
            markets.insert(market.condition_id.clone(), market.clone());
        } // drop lock

        let markets = self.markets.read().await;
        let data = MarketData {
            markets: markets.clone(),
        };
        self.save_to_file("markets.json", &data).await?;

        Ok(())
    }

    async fn save_event(&self, event: &MarketEvent) -> Result<()> {
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
            // Keep only last 1000 events
            if events.len() > 1000 {
                events.remove(0);
            }
        } // drop lock

        let events = self.events.read().await;
        let data = EventData {
            events: events.clone(),
        };
        self.save_to_file("events.json", &data).await?;

        Ok(())
    }

    async fn save_price_history(
        &self,
        condition_id: &str,
        outcome_prices: Option<&str>,
        volume: Option<&str>,
    ) -> Result<()> {
        let mut history = self.price_history.write().await;
        let entry = history
            .entry(condition_id.to_string())
            .or_insert_with(Vec::new);

        entry.push((
            outcome_prices.unwrap_or("").to_string(),
            volume.unwrap_or("").to_string(),
            Utc::now(),
        ));

        // Keep only last 1000 entries
        if entry.len() > 1000 {
            entry.remove(0);
        }

        // Note: For a full implementation, we would save this to disk too.
        // For now, we'll just keep it in memory as per the "simple" requirement,
        // or we could dump all history to a big json file, but that might be slow.
        // Let's stick to in-memory for history for this iteration unless requested otherwise.

        Ok(())
    }

    async fn get_market_count(&self) -> Result<i64> {
        let markets = self.markets.read().await;
        Ok(markets.len() as i64)
    }

    async fn get_event_count(&self) -> Result<i64> {
        let events = self.events.read().await;
        Ok(events.len() as i64)
    }

    async fn get_price_history(
        &self,
        condition_id: &str,
        limit: i32,
    ) -> Result<Vec<(String, String, DateTime<Utc>)>> {
        let history = self.price_history.read().await;
        if let Some(entries) = history.get(condition_id) {
            let start = if entries.len() > limit as usize {
                entries.len() - limit as usize
            } else {
                0
            };
            Ok(entries[start..].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_recent_events(
        &self,
        limit: i32,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>)>> {
        let events = self.events.read().await;
        let start = if events.len() > limit as usize {
            events.len() - limit as usize
        } else {
            0
        };

        let result = events[start..]
            .iter()
            .rev()
            .map(|e| {
                let event_type_str = match e.event_type {
                    EventType::NewMarket => "NewMarket",
                    EventType::PriceChange => "PriceChange",
                    EventType::VolumeUpdate => "VolumeUpdate",
                    EventType::MarketClosed => "MarketClosed",
                };
                (
                    event_type_str.to_string(),
                    e.market.question.clone(),
                    e.market.outcome_prices.clone().unwrap_or_default(),
                    e.timestamp,
                )
            })
            .collect();

        Ok(result)
    }

    async fn get_market(&self, condition_id: &str) -> Result<Option<Market>> {
        let markets = self.markets.read().await;
        Ok(markets.get(condition_id).cloned())
    }

    async fn get_all_market_ids(&self) -> Result<Vec<String>> {
        let markets = self.markets.read().await;
        Ok(markets.keys().cloned().collect())
    }

    async fn get_event_stats(&self) -> Result<HashMap<String, i64>> {
        let events = self.events.read().await;
        let mut stats = HashMap::new();

        for event in events.iter() {
            let event_type_str = match event.event_type {
                EventType::NewMarket => "NewMarket",
                EventType::PriceChange => "PriceChange",
                EventType::VolumeUpdate => "VolumeUpdate",
                EventType::MarketClosed => "MarketClosed",
            };
            *stats.entry(event_type_str.to_string()).or_insert(0) += 1;
        }

        stats.insert("Total".to_string(), events.len() as i64);
        Ok(stats)
    }
}
