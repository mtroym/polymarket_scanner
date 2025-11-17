use crate::error::{Result, ScannerError};
use crate::types::{Market, MarketEvent, EventType};
use chrono::{DateTime, Utc};
use log::{info, debug};
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
    
    /// 初始化 Redis（可选，Redis 不需要 schema）
    pub async fn init_schema(&self) -> Result<()> {
        info!("Redis 初始化完成（无需创建表结构）");
        Ok(())
    }
    
    /// 保存或更新市场数据
    pub async fn save_market(&self, market: &Market) -> Result<()> {
        let mut conn = self.conn.clone();
        let key = format!("market:{}", market.condition_id);
        let now = Utc::now().to_rfc3339();
        
        // 检查市场是否已存在
        let exists: bool = conn.exists(&key)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Redis 查询失败: {}", e)))?;
        
        let first_seen = if exists {
            // 获取原有的 first_seen_at
            conn.hget::<_, _, Option<String>>(&key, "first_seen_at")
                .await
                .unwrap_or(Some(now.clone()))
                .unwrap_or_else(|| now.clone())
        } else {
            now.clone()
        };
        
        // 使用 Hash 存储市场数据
        let _: () = conn.hset_multiple(&key, &[
            ("condition_id", market.condition_id.as_str()),
            ("question_id", market.question_id.as_deref().unwrap_or("")),
            ("question", &market.question),
            ("description", market.description.as_deref().unwrap_or("")),
            ("market_slug", market.market_slug.as_deref().unwrap_or("")),
            ("outcomes", &market.outcomes),
            ("outcome_prices", &market.outcome_prices),
            ("volume", market.volume.as_deref().unwrap_or("")),
            ("liquidity", market.liquidity.as_deref().unwrap_or("")),
            ("end_date", market.end_date.as_deref().unwrap_or("")),
            ("active", &market.active.map(|b| if b { "1" } else { "0" }).unwrap_or("0")),
            ("closed", &market.closed.map(|b| if b { "1" } else { "0" }).unwrap_or("0")),
            ("first_seen_at", &first_seen),
            ("last_updated_at", &now),
        ])
        .await
        .map_err(|e| ScannerError::ConfigError(format!("保存市场失败: {}", e)))?;
        
        // 添加到市场列表集合
        let _: () = conn.sadd("markets:all", &market.condition_id)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("添加到市场集合失败: {}", e)))?;
        
        if !exists {
            info!("保存新市场: {}", market.question);
        } else {
            debug!("更新市场: {}", market.condition_id);
        }
        
        Ok(())
    }
    
    /// 保存市场事件
    pub async fn save_event(&self, event: &MarketEvent) -> Result<()> {
        let mut conn = self.conn.clone();
        
        let event_type_str = match event.event_type {
            EventType::NewMarket => "NewMarket",
            EventType::PriceChange => "PriceChange",
            EventType::VolumeUpdate => "VolumeUpdate",
            EventType::MarketClosed => "MarketClosed",
        };
        
        // 事件数据序列化为 JSON
        let event_data = serde_json::json!({
            "condition_id": event.market.condition_id,
            "event_type": event_type_str,
            "question": event.market.question,
            "outcomes": event.market.outcomes,
            "outcome_prices": event.market.outcome_prices,
            "volume": event.market.volume,
            "liquidity": event.market.liquidity,
            "timestamp": event.timestamp.to_rfc3339(),
        });
        
        let event_json = serde_json::to_string(&event_data)
            .map_err(|e| ScannerError::JsonError(e))?;
        
        // 添加到全局事件列表（保留最近 1000 条）
        let _: () = conn.lpush("events:recent", &event_json)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("保存事件失败: {}", e)))?;
        
        let _: () = conn.ltrim("events:recent", 0, 999)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("修剪事件列表失败: {}", e)))?;
        
        // 添加到特定市场的事件列表
        // let market_events_key = format!("market:{}:events", event.market.condition_id);
        // let _: () = conn.lpush(&market_events_key, &event_json)
        //     .await
        //     .map_err(|e| ScannerError::ConfigError(format!("保存市场事件失败: {}", e)))?;
        
        // let _: () = conn.ltrim(&market_events_key, 0, 99)
        //     .await
        //     .ok();
        
        // // 增加事件计数器
        // let counter_key = format!("stats:events:{}", event_type_str);
        // let _: () = conn.incr(&counter_key, 1)
        //     .await
        //     .ok();
        
        // let _: () = conn.incr("stats:events:total", 1)
        //     .await
        //     .ok();
        
        // debug!("保存事件: {} - {}", event_type_str, event.market.question);
        Ok(())
    }
    
    /// 保存价格历史
    pub async fn save_price_history(
        &self,
        condition_id: &str,
        outcome_prices: &str,
        volume: Option<&str>,
    ) -> Result<()> {
        let mut conn = self.conn.clone();
        let now = Utc::now();
        let timestamp_ms = now.timestamp_millis() as f64;
        
        // 价格历史数据
        let history_data = serde_json::json!({
            "outcome_prices": outcome_prices,
            "volume": volume.unwrap_or(""),
            "timestamp": now.to_rfc3339(),
        });
        
        let history_json = serde_json::to_string(&history_data)
            .map_err(|e| ScannerError::JsonError(e))?;
        
        // 使用 Sorted Set 存储价格历史（按时间戳排序）
        let key = format!("market:{}:price_history", condition_id);
        let _: () = conn.zadd(&key, &history_json, timestamp_ms)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("保存价格历史失败: {}", e)))?;
        
        // 只保留最近 1000 条记录
        // let count: i64 = conn.zcard(&key)
        //     .await
        //     .unwrap_or(0);
        
        // if count > 1000 {
        //     let _: () = conn.zremrangebyrank(&key, 0, count - 1001)
        //         .await
        //         .ok();
        // }
        
        Ok(())
    }
    
    /// 获取市场总数
    pub async fn get_market_count(&self) -> Result<i64> {
        let mut conn = self.conn.clone();
        let count: i64 = conn.scard("markets:all")
            .await
            .map_err(|e| ScannerError::ConfigError(format!("查询市场总数失败: {}", e)))?;
        
        Ok(count)
    }
    
    /// 获取事件总数
    pub async fn get_event_count(&self) -> Result<i64> {
        let mut conn = self.conn.clone();
        let count: i64 = conn.get("stats:events:total")
            .await
            .unwrap_or(0);
        
        Ok(count)
    }
    
    /// 获取特定市场的价格历史
    pub async fn get_price_history(
        &self,
        condition_id: &str,
        limit: i32,
    ) -> Result<Vec<(String, String, DateTime<Utc>)>> {
        let mut conn = self.conn.clone();
        let key = format!("market:{}:price_history", condition_id);
        
        // 从 Sorted Set 中获取最近的记录（倒序）
        let results: Vec<String> = conn.zrevrange(&key, 0, (limit - 1) as isize)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("查询价格历史失败: {}", e)))?;
        
        let mut history = Vec::new();
        for json_str in results {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let prices = data["outcome_prices"].as_str().unwrap_or("").to_string();
                let volume = data["volume"].as_str().unwrap_or("").to_string();
                let timestamp_str = data["timestamp"].as_str().unwrap_or("");
                
                let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                    .unwrap_or_else(|_| DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap())
                    .with_timezone(&Utc);
                
                history.push((prices, volume, timestamp));
            }
        }
        
        Ok(history)
    }
    
    /// 获取最近的事件
    pub async fn get_recent_events(&self, limit: i32) -> Result<Vec<(String, String, String, DateTime<Utc>)>> {
        let mut conn = self.conn.clone();
        
        let results: Vec<String> = conn.lrange("events:recent", 0, (limit - 1) as isize)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("查询最近事件失败: {}", e)))?;
        
        let mut events = Vec::new();
        for json_str in results {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let event_type = data["event_type"].as_str().unwrap_or("").to_string();
                let question = data["question"].as_str().unwrap_or("").to_string();
                let prices = data["outcome_prices"].as_str().unwrap_or("").to_string();
                let timestamp_str = data["timestamp"].as_str().unwrap_or("");
                
                let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                    .unwrap_or_else(|_| DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap())
                    .with_timezone(&Utc);
                
                events.push((event_type, question, prices, timestamp));
            }
        }
        
        Ok(events)
    }
    
    /// 获取市场详情
    #[allow(dead_code)]
    pub async fn get_market(&self, condition_id: &str) -> Result<Option<Market>> {
        let mut conn = self.conn.clone();
        let key = format!("market:{}", condition_id);
        
        let exists: bool = conn.exists(&key)
            .await
            .map_err(|e| ScannerError::ConfigError(format!("Redis 查询失败: {}", e)))?;
        
        if !exists {
            return Ok(None);
        }
        
        let data: Vec<String> = conn.hgetall(&key)
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
            question_id: map.get("question_id").and_then(|s| if s.is_empty() { None } else { Some(s.clone()) }),
            question: map.get("question").cloned().unwrap_or_default(),
            description: map.get("description").and_then(|s| if s.is_empty() { None } else { Some(s.clone()) }),
            market_slug: map.get("market_slug").and_then(|s| if s.is_empty() { None } else { Some(s.clone()) }),
            outcomes: map.get("outcomes").cloned().unwrap_or_default(),
            outcome_prices: map.get("outcome_prices").cloned().unwrap_or_default(),
            volume: map.get("volume").and_then(|s| if s.is_empty() { None } else { Some(s.clone()) }),
            liquidity: map.get("liquidity").and_then(|s| if s.is_empty() { None } else { Some(s.clone()) }),
            end_date: map.get("end_date").and_then(|s| if s.is_empty() { None } else { Some(s.clone()) }),
            active: map.get("active").and_then(|s| Some(s == "1")),
            closed: map.get("closed").and_then(|s| Some(s == "1")),
        };
        
        Ok(Some(market))
    }
    
    /// 获取所有市场 ID
    #[allow(dead_code)]
    pub async fn get_all_market_ids(&self) -> Result<Vec<String>> {
        let mut conn = self.conn.clone();
        let ids: Vec<String> = conn.smembers("markets:all")
            .await
            .map_err(|e| ScannerError::ConfigError(format!("获取市场列表失败: {}", e)))?;
        
        Ok(ids)
    }
    
    /// 获取事件统计
    #[allow(dead_code)]
    pub async fn get_event_stats(&self) -> Result<std::collections::HashMap<String, i64>> {
        let mut conn = self.conn.clone();
        let mut stats = std::collections::HashMap::new();
        
        let event_types = ["NewMarket", "PriceChange", "VolumeUpdate", "MarketClosed"];
        
        for event_type in &event_types {
            let key = format!("stats:events:{}", event_type);
            let count: i64 = conn.get(&key)
                .await
                .unwrap_or(0);
            stats.insert(event_type.to_string(), count);
        }
        
        let total: i64 = conn.get("stats:events:total")
            .await
            .unwrap_or(0);
        stats.insert("Total".to_string(), total);
        
        Ok(stats)
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
