use crate::error::{Result, ScannerError};
use crate::storage::Storage;
use crate::types::Market;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::info;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

pub struct Database {
    conn: ConnectionManager,
}

impl Database {
    /// 创建 Redis 连接
    pub async fn new(redis_url: &str) -> Result<Self> {
        info!("连接 Redis: {}", redis_url);

        let client = redis::Client::open(redis_url)
            .map_err(|e| ScannerError::ConfigError(format!("Redis 客户端创建失败: {}", e)))?;

        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Redis 连接失败: {}", e)))?;

        info!("Redis 连接成功");
        Ok(Self { conn })
    }

    /// 清空所有数据（慎用）
    #[allow(dead_code)]
    pub async fn flush_all(&self) -> Result<()> {
        let mut conn = self.conn.clone();
        redis::cmd("FLUSHDB")
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("清空数据库失败: {}", e)))?;

        info!("Redis 数据库已清空");
        Ok(())
    }
}

#[async_trait]
impl Storage for Database {
    /// 初始化 Redis（可选，Redis 不需要 schema）
    async fn init(&self) -> Result<()> {
        info!("Redis 初始化完成（无需创建表结构）");
        Ok(())
    }

    /// 保存或更新市场数据
    async fn save_market(&self, market: &Market) -> Result<()> {
        self.save_markets(vec![market.clone()]).await
    }

    async fn save_markets(&self, markets: Vec<Market>) -> Result<()> {
        let mut conn = self.conn.clone();
        let mut pipe = redis::pipe();
        let now = Utc::now().to_rfc3339();

        for market in markets {
            let key = format!("market:{}", market.condition_id);

            // Note: In a pipeline, we can't easily check for existence and conditionally update
            // 'first_seen_at' without a Lua script or multiple round trips.
            // For simplicity and performance in batch mode, we'll assume 'first_seen_at' is 'now'
            // if not present, or we could fetch all keys first (but that's slow).
            // Alternatively, we can use HSETNX for 'first_seen_at' if we want to preserve it,
            // but HSETNX is for a single field.
            // Let's just set 'first_seen_at' to 'now' if it's a new market.
            // Actually, Redis HSET overwrites. To preserve 'first_seen_at', we'd need to read it.
            // Reading in a loop is bad.
            // Optimization: Just set 'last_updated_at'. If we really need 'first_seen_at',
            // we should use HSETNX for that specific field in a separate command or assume the caller handles it.
            // Given the constraints, let's just write everything. If 'first_seen_at' is overwritten, so be it for now,
            // or we can try to read it if we want to be perfect, but batching is about speed.
            // Let's stick to the previous logic but adapted for pipeline?
            // No, previous logic did a read for every market. That defeats the purpose of batching.
            // Let's just write. If we want to preserve 'first_seen_at', we can use HSETNX for it.

            pipe.hset_multiple(
                &key,
                &[
                    ("condition_id", market.condition_id.as_str()),
                    ("question_id", market.question_id.as_deref().unwrap_or("")),
                    ("question", &market.question),
                    ("description", market.description.as_deref().unwrap_or("")),
                    ("market_slug", market.market_slug.as_deref().unwrap_or("")),
                    ("outcomes", &market.outcomes),
                    (
                        "outcome_prices",
                        market.outcome_prices.as_deref().unwrap_or(""),
                    ),
                    ("volume", market.volume.as_deref().unwrap_or("")),
                    ("liquidity", market.liquidity.as_deref().unwrap_or("")),
                    ("end_date", market.end_date.as_deref().unwrap_or("")),
                    (
                        "active",
                        &market
                            .active
                            .map(|b| if b { "1" } else { "0" })
                            .unwrap_or("0"),
                    ),
                    (
                        "closed",
                        &market
                            .closed
                            .map(|b| if b { "1" } else { "0" })
                            .unwrap_or("0"),
                    ),
                    ("last_updated_at", &now),
                ],
            );

            // Use HSETNX for first_seen_at to only set it if it doesn't exist
            pipe.hset_nx(&key, "first_seen_at", &now);

            // Add to set
            pipe.sadd("markets:all", &market.condition_id);
        }

        let _: () = pipe
            .query_async(&mut conn)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Batch save markets failed: {}", e)))?;

        Ok(())
    }

    /// 保存价格历史
    async fn save_price_history(
        &self,
        condition_id: &str,
        outcome_prices: Option<&str>,
        volume: Option<&str>,
    ) -> Result<()> {
        let mut conn = self.conn.clone();
        let now = Utc::now();
        let timestamp_ms = now.timestamp_millis() as f64;

        // 价格历史数据
        let history_data = serde_json::json!({
            "outcome_prices": outcome_prices.unwrap_or(""),
            "volume": volume.unwrap_or(""),
            "timestamp": now.to_rfc3339(),
        });

        let history_json =
            serde_json::to_string(&history_data).map_err(|e| ScannerError::JsonError(e))?;

        // 使用 Sorted Set 存储价格历史（按时间戳排序）
        let key = format!("market:{}:price_history", condition_id);
        let _: () = conn
            .zadd(&key, &history_json, timestamp_ms)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("保存价格历史失败: {}", e)))?;

        Ok(())
    }

    /// 获取市场总数
    async fn get_market_count(&self) -> Result<i64> {
        let mut conn = self.conn.clone();
        let count: i64 = conn
            .scard("markets:all")
            .await
            .map_err(|e| ScannerError::ConfigError(format!("查询市场总数失败: {}", e)))?;

        Ok(count)
    }

    /// 获取特定市场的价格历史
    async fn get_price_history(
        &self,
        condition_id: &str,
        limit: i32,
    ) -> Result<Vec<(String, String, DateTime<Utc>)>> {
        let mut conn = self.conn.clone();
        let key = format!("market:{}:price_history", condition_id);

        // 从 Sorted Set 中获取最近的记录（倒序）
        let results: Vec<String> = conn
            .zrevrange(&key, 0, (limit - 1) as isize)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("查询价格历史失败: {}", e)))?;

        let mut history = Vec::new();
        for json_str in results {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let prices = data["outcome_prices"].as_str().unwrap_or("").to_string();
                let volume = data["volume"].as_str().unwrap_or("").to_string();
                let timestamp_str = data["timestamp"].as_str().unwrap_or("");

                let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                    .unwrap_or_else(|_| {
                        DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap()
                    })
                    .with_timezone(&Utc);

                history.push((prices, volume, timestamp));
            }
        }

        Ok(history)
    }

    /// 获取市场详情
    async fn get_market(&self, condition_id: &str) -> Result<Option<Market>> {
        let mut conn = self.conn.clone();
        let key = format!("market:{}", condition_id);

        let exists: bool = conn
            .exists(&key)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Redis 查询失败: {}", e)))?;

        if !exists {
            return Ok(None);
        }

        let data: Vec<String> = conn
            .hgetall(&key)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("获取市场数据失败: {}", e)))?;

        // Redis HGETALL 返回 [key, value, key, value, ...]
        let mut map = std::collections::HashMap::new();
        for i in (0..data.len()).step_by(2) {
            if i + 1 < data.len() {
                map.insert(data[i].clone(), data[i + 1].clone());
            }
        }

        let market = Market {
            condition_id: map.get("condition_id").cloned().unwrap_or_default(),
            question_id: map.get("question_id").and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }),
            question: map.get("question").cloned().unwrap_or_default(),
            description: map.get("description").and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }),
            market_slug: map.get("market_slug").and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }),
            outcomes: map.get("outcomes").cloned().unwrap_or("[]".to_string()),
            outcome_prices: Some(
                map.get("outcome_prices")
                    .cloned()
                    .unwrap_or("[]".to_string()),
            ),
            volume: map
                .get("volume")
                .and_then(|s| if s.is_empty() { None } else { Some(s.clone()) }),
            liquidity: map.get("liquidity").and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }),
            end_date: map.get("end_date").and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }),
            active: map.get("active").and_then(|s| Some(s == "1")),
            closed: map.get("closed").and_then(|s| Some(s == "1")),
        };

        Ok(Some(market))
    }

    /// 获取所有市场 ID
    async fn get_all_market_ids(&self) -> Result<Vec<String>> {
        let mut conn = self.conn.clone();
        let ids: Vec<String> = conn
            .smembers("markets:all")
            .await
            .map_err(|e| ScannerError::ConfigError(format!("获取市场列表失败: {}", e)))?;

        Ok(ids)
    }
}
