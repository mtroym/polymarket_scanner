use crate::error::{Result, ScannerError};
use crate::types::{Market, MarketEvent, EventType};
use chrono::{DateTime, Utc};
use log::{info, debug};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// 创建数据库连接
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("连接数据库: {}", database_url);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("数据库连接失败: {}", e)))?;
        
        Ok(Self { pool })
    }
    
    /// 初始化数据库表
    pub async fn init_schema(&self) -> Result<()> {
        info!("初始化数据库表结构");
        
        // 创建市场表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS markets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                condition_id TEXT NOT NULL UNIQUE,
                question_id TEXT,
                question TEXT NOT NULL,
                description TEXT,
                market_slug TEXT,
                outcomes TEXT NOT NULL,
                outcome_prices TEXT NOT NULL,
                volume TEXT,
                liquidity TEXT,
                end_date TEXT,
                active INTEGER,
                closed INTEGER,
                first_seen_at TEXT NOT NULL,
                last_updated_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ScannerError::ConfigError(format!("创建 markets 表失败: {}", e)))?;
        
        // 创建市场事件表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS market_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                condition_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                question TEXT NOT NULL,
                outcomes TEXT,
                outcome_prices TEXT,
                volume TEXT,
                liquidity TEXT,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (condition_id) REFERENCES markets(condition_id)
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ScannerError::ConfigError(format!("创建 market_events 表失败: {}", e)))?;
        
        // 创建价格历史表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS price_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                condition_id TEXT NOT NULL,
                outcome_prices TEXT NOT NULL,
                volume TEXT,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (condition_id) REFERENCES markets(condition_id)
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ScannerError::ConfigError(format!("创建 price_history 表失败: {}", e)))?;
        
        // 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_markets_condition_id ON markets(condition_id)")
            .execute(&self.pool)
            .await
            .ok();
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_condition_id ON market_events(condition_id)")
            .execute(&self.pool)
            .await
            .ok();
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON market_events(timestamp)")
            .execute(&self.pool)
            .await
            .ok();
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_price_history_condition_id ON price_history(condition_id)")
            .execute(&self.pool)
            .await
            .ok();
        
        info!("数据库表结构初始化完成");
        Ok(())
    }
    
    /// 保存或更新市场数据
    pub async fn save_market(&self, market: &Market) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        // 检查市场是否已存在
        let exists: bool = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM markets WHERE condition_id = ?)"
        )
        .bind(&market.condition_id)
        .fetch_one(&self.pool)
        .await
        .map(|row| row.get(0))
        .unwrap_or(false);
        
        if exists {
            // 更新现有市场
            sqlx::query(
                r#"
                UPDATE markets SET
                    question_id = ?,
                    question = ?,
                    description = ?,
                    market_slug = ?,
                    outcomes = ?,
                    outcome_prices = ?,
                    volume = ?,
                    liquidity = ?,
                    end_date = ?,
                    active = ?,
                    closed = ?,
                    last_updated_at = ?
                WHERE condition_id = ?
                "#
            )
            .bind(&market.question_id)
            .bind(&market.question)
            .bind(&market.description)
            .bind(&market.market_slug)
            .bind(&market.outcomes)
            .bind(&market.outcome_prices)
            .bind(&market.volume)
            .bind(&market.liquidity)
            .bind(&market.end_date)
            .bind(market.active.map(|b| b as i32))
            .bind(market.closed.map(|b| b as i32))
            .bind(&now)
            .bind(&market.condition_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("更新市场失败: {}", e)))?;
            
            debug!("更新市场: {}", market.condition_id);
        } else {
            // 插入新市场
            sqlx::query(
                r#"
                INSERT INTO markets (
                    condition_id, question_id, question, description, market_slug,
                    outcomes, outcome_prices, volume, liquidity, end_date,
                    active, closed, first_seen_at, last_updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&market.condition_id)
            .bind(&market.question_id)
            .bind(&market.question)
            .bind(&market.description)
            .bind(&market.market_slug)
            .bind(&market.outcomes)
            .bind(&market.outcome_prices)
            .bind(&market.volume)
            .bind(&market.liquidity)
            .bind(&market.end_date)
            .bind(market.active.map(|b| b as i32))
            .bind(market.closed.map(|b| b as i32))
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("插入市场失败: {}", e)))?;
            
            info!("保存新市场: {}", market.question);
        }
        
        Ok(())
    }
    
    /// 保存市场事件
    pub async fn save_event(&self, event: &MarketEvent) -> Result<()> {
        let event_type_str = match event.event_type {
            EventType::NewMarket => "NewMarket",
            EventType::PriceChange => "PriceChange",
            EventType::VolumeUpdate => "VolumeUpdate",
            EventType::MarketClosed => "MarketClosed",
        };
        
        sqlx::query(
            r#"
            INSERT INTO market_events (
                condition_id, event_type, question, outcomes, outcome_prices,
                volume, liquidity, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&event.market.condition_id)
        .bind(event_type_str)
        .bind(&event.market.question)
        .bind(&event.market.outcomes)
        .bind(&event.market.outcome_prices)
        .bind(&event.market.volume)
        .bind(&event.market.liquidity)
        .bind(event.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ScannerError::ConfigError(format!("保存事件失败: {}", e)))?;
        
        debug!("保存事件: {} - {}", event_type_str, event.market.question);
        Ok(())
    }
    
    /// 保存价格历史
    pub async fn save_price_history(&self, condition_id: &str, outcome_prices: &str, volume: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT INTO price_history (condition_id, outcome_prices, volume, timestamp)
            VALUES (?, ?, ?, ?)
            "#
        )
        .bind(condition_id)
        .bind(outcome_prices)
        .bind(volume)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| ScannerError::ConfigError(format!("保存价格历史失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取市场总数
    pub async fn get_market_count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM markets")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("查询市场总数失败: {}", e)))?;
        
        Ok(count.0)
    }
    
    /// 获取事件总数
    pub async fn get_event_count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM market_events")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("查询事件总数失败: {}", e)))?;
        
        Ok(count.0)
    }
    
    /// 获取特定市场的价格历史
    pub async fn get_price_history(&self, condition_id: &str, limit: i32) -> Result<Vec<(String, String, DateTime<Utc>)>> {
        let rows = sqlx::query(
            r#"
            SELECT outcome_prices, volume, timestamp
            FROM price_history
            WHERE condition_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#
        )
        .bind(condition_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ScannerError::ConfigError(format!("查询价格历史失败: {}", e)))?;
        
        let mut history = Vec::new();
        for row in rows {
            let prices: String = row.get("outcome_prices");
            let volume: Option<String> = row.get("volume");
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .unwrap_or_else(|_| DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap())
                .with_timezone(&Utc);
            
            history.push((prices, volume.unwrap_or_default(), timestamp));
        }
        
        Ok(history)
    }
    
    /// 获取最近的事件
    pub async fn get_recent_events(&self, limit: i32) -> Result<Vec<(String, String, String, DateTime<Utc>)>> {
        let rows = sqlx::query(
            r#"
            SELECT event_type, question, outcome_prices, timestamp
            FROM market_events
            ORDER BY timestamp DESC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ScannerError::ConfigError(format!("查询最近事件失败: {}", e)))?;
        
        let mut events = Vec::new();
        for row in rows {
            let event_type: String = row.get("event_type");
            let question: String = row.get("question");
            let prices: Option<String> = row.get("outcome_prices");
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .unwrap_or_else(|_| DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap())
                .with_timezone(&Utc);
            
            events.push((event_type, question, prices.unwrap_or_default(), timestamp));
        }
        
        Ok(events)
    }
}

