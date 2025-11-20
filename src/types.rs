use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    #[serde(rename = "conditionId")]
    pub condition_id: String,

    #[serde(rename = "questionID")]
    pub question_id: Option<String>,

    pub question: String,

    pub description: Option<String>,

    #[serde(rename = "marketSlug")]
    pub market_slug: Option<String>,

    pub outcomes: String,

    #[serde(rename = "outcomePrices")]
    pub outcome_prices: Option<String>,

    pub volume: Option<String>,

    pub liquidity: Option<String>,

    #[serde(rename = "endDate")]
    pub end_date: Option<String>,

    pub active: Option<bool>,

    pub closed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketEvent {
    pub market: Market,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    NewMarket,
    PriceChange,
    VolumeUpdate,
    MarketClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketsResponse {
    pub data: Vec<Market>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistory {
    pub t: i64, // timestamp
    pub p: f64, // price
}
