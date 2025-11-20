use crate::error::Result;
use crate::types::{Market, MarketEvent};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[async_trait]
pub trait Storage: Send + Sync {
    /// Initialize the storage (e.g. create tables, ensure directories exist)
    async fn init(&self) -> Result<()>;

    /// Save or update a market
    async fn save_market(&self, market: &Market) -> Result<()>;

    /// Save a market event
    async fn save_event(&self, event: &MarketEvent) -> Result<()>;

    /// Save price history for a market
    async fn save_price_history(
        &self,
        condition_id: &str,
        outcome_prices: Option<&str>,
        volume: Option<&str>,
    ) -> Result<()>;

    /// Get total number of tracked markets
    async fn get_market_count(&self) -> Result<i64>;

    /// Get total number of recorded events
    async fn get_event_count(&self) -> Result<i64>;

    /// Get price history for a market
    async fn get_price_history(
        &self,
        condition_id: &str,
        limit: i32,
    ) -> Result<Vec<(String, String, DateTime<Utc>)>>;

    /// Get recent events
    async fn get_recent_events(
        &self,
        limit: i32,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>)>>;

    /// Get a specific market
    async fn get_market(&self, condition_id: &str) -> Result<Option<Market>>;

    /// Get all market IDs
    async fn get_all_market_ids(&self) -> Result<Vec<String>>;

    /// Get event statistics
    async fn get_event_stats(&self) -> Result<HashMap<String, i64>>;
}
