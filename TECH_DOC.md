# Polymarket Scanner Technical Documentation

## 1. Project Overview

Polymarket Scanner is a Rust-based application designed to monitor Polymarket markets in real-time. It fetches market data, detects changes (price, volume, etc.), and stores the history.

## 2. Architecture

The project follows a modular architecture:

- **Main Entry (`main.rs`)**: Initializes the application, sets up logging, and starts the scanner.
- **API Client (`api.rs`)**: Handles communication with the Polymarket API.
- **Scanner (`scanner.rs`)**: Core logic for monitoring markets and detecting events.
- **Database Layer**: Handles data persistence.
- **Types (`types.rs`)**: Defines data structures for markets and events.

### Data Flow

1.  **Fetch**: The scanner periodically fetches market data from the Polymarket API.
2.  **Detect**: It compares the new data with the cached state to detect events (New Market, Price Change, Volume Update, Market Closed).
3.  **Handle**: Detected events are processed (logged, printed) and sent to the storage layer.
4.  **Store**: The storage layer persists markets, events, and price history.

## 3. Database Implementation

The project currently supports multiple database backends. The goal is to provide a flexible storage interface.

### 3.1. Current Implementations

-   **Redis (`database.rs`)**: Uses Redis for high-performance storage. It stores markets as Hashes, events as Lists, and price history as Sorted Sets.
-   **SQLite (`db.rs`)**: Uses SQLite for relational storage. It has tables for `markets`, `market_events`, and `price_history`.

### 3.2. Proposed JSON File Storage

To support simple, zero-dependency persistence, we are adding a JSON file-based storage backend.

#### Design

-   **File Structure**:
    -   `markets.json`: Stores the latest state of all tracked markets.
    -   `events.json`: Stores a log of market events.
    -   `history/`: A directory containing price history files, potentially one per market or aggregated.
-   **In-Memory Cache**: The JSON storage will load data into memory on startup and periodically flush changes to disk to ensure performance.
-   **Persistence Strategy**:
    -   **Atomic Writes**: Use temporary files and rename to ensure data integrity during writes.
    -   **Buffering**: Batch writes to avoid excessive disk I/O.

#### Interface

We will introduce a `Storage` trait to standardize the database interface:

```rust
#[async_trait]
pub trait Storage: Send + Sync {
    async fn init(&self) -> Result<()>;
    async fn save_market(&self, market: &Market) -> Result<()>;
    async fn save_event(&self, event: &MarketEvent) -> Result<()>;
    async fn save_price_history(&self, condition_id: &str, outcome_prices: &str, volume: Option<&str>) -> Result<()>;
    async fn get_market_count(&self) -> Result<i64>;
    async fn get_event_count(&self) -> Result<i64>;
    // ... other methods
}
```

## 4. Configuration

The application is configured via environment variables:

-   `REDIS_URL`: URL for Redis connection.
-   `DATABASE_URL`: URL for SQLite connection.
-   `STORAGE_TYPE`: Select storage backend (`redis`, `sqlite`, `json`).
-   `JSON_DB_PATH`: Path for JSON database files (default: `./data`).

## 5. Future Improvements

-   **WebSocket Integration**: For real-time updates instead of polling.
-   **Advanced Analytics**: Calculate stats like volatility, volume trends.
-   **Web Dashboard**: Visualize the data stored in the database.
