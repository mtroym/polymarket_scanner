pub mod api;
pub mod scanner;
pub mod types;
pub mod error;
pub mod database;

pub use api::PolymarketClient;
pub use scanner::MarketScanner;
pub use types::{Market, MarketEvent, EventType};
pub use error::{ScannerError, Result};
pub use database::Database;
