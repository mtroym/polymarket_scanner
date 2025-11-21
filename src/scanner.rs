use crate::api::PolymarketClient;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::{EventType, Market, MarketEvent};
use chrono::Utc;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct MarketScanner {
    client: PolymarketClient,
    database: Option<Arc<dyn Storage + Send + Sync>>,
    tracked_markets: HashMap<String, Market>,
}

impl MarketScanner {
    #[allow(dead_code)]
    pub fn new(client: PolymarketClient) -> Self {
        Self {
            client,
            database: None,
            tracked_markets: HashMap::new(),
        }
    }

    /// 创建带数据库支持的扫描器
    pub fn with_database(
        client: PolymarketClient,
        database: Arc<dyn Storage + Send + Sync>,
    ) -> Self {
        Self {
            client,
            database: Some(database),
            tracked_markets: HashMap::new(),
        }
    }

    /// 开始扫描市场
    pub async fn start_scanning(&self, interval: Duration) -> Result<()> {
        info!("开始扫描 Polymarket 市场，扫描间隔: {:?}", interval);

        // 如果有数据库，先加载已保存的市场
        let mut tracked_markets = if let Some(db) = &self.database {
            info!("正在从数据库加载市场数据...");
            let mut markets = HashMap::new();
            if let Ok(ids) = db.get_all_market_ids().await {
                for id in ids {
                    if let Ok(Some(market)) = db.get_market(&id).await {
                        markets.insert(id, market);
                    }
                }
            }
            info!("已加载 {} 个市场", markets.len());
            markets
        } else {
            self.tracked_markets.clone()
        };

        loop {
            match self.scan_markets(&mut tracked_markets).await {
                Ok(events) => {
                    if !events.is_empty() {
                        info!("检测到 {} 个市场事件", events.len());
                        for event in events {
                            self.handle_event(event);
                        }
                    } else {
                        debug!("本轮扫描未发现新事件");
                    }
                }
                Err(e) => {
                    error!("扫描错误: {}", e);
                }
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// 扫描市场并检测变化
    async fn scan_markets(
        &self,
        tracked_markets: &mut HashMap<String, Market>,
    ) -> Result<Vec<MarketEvent>> {
        let markets = self.client.get_markets(Some(50)).await?;
        let mut events = Vec::new();

        for market in markets {
            let condition_id = market.condition_id.clone();

            if let Some(old_market) = tracked_markets.get(&condition_id) {
                // 检测价格变化
                if market.outcome_prices != old_market.outcome_prices {
                    info!(
                        "市场价格变化 [{}]: {:?} -> {:?}",
                        market.question, old_market.outcome_prices, market.outcome_prices
                    );

                    events.push(MarketEvent {
                        market: market.clone(),
                        timestamp: Utc::now(),
                        event_type: EventType::PriceChange,
                    });
                }

                // 检测成交量变化
                if market.volume != old_market.volume {
                    debug!(
                        "市场成交量变化 [{}]: {:?} -> {:?}",
                        market.question, old_market.volume, market.volume
                    );

                    events.push(MarketEvent {
                        market: market.clone(),
                        timestamp: Utc::now(),
                        event_type: EventType::VolumeUpdate,
                    });
                }

                // 检测市场关闭
                if market.closed == Some(true) && old_market.closed != Some(true) {
                    info!("市场已关闭 [{}]", market.question);

                    events.push(MarketEvent {
                        market: market.clone(),
                        timestamp: Utc::now(),
                        event_type: EventType::MarketClosed,
                    });
                }

                // 更新追踪的市场
                tracked_markets.insert(condition_id, market);
            } else {
                // 新市场
                info!("发现新市场: {}", market.question);
                info!("  - 结果选项: {:?}", market.outcomes);
                info!("  - 当前价格: {:?}", market.outcome_prices);
                if let Some(volume) = &market.volume {
                    info!("  - 成交量: {}", volume);
                }

                events.push(MarketEvent {
                    market: market.clone(),
                    timestamp: Utc::now(),
                    event_type: EventType::NewMarket,
                });

                tracked_markets.insert(condition_id, market);
            }
        }

        Ok(events)
    }

    /// 处理市场事件
    fn handle_event(&self, event: MarketEvent) {
        match event.event_type {
            EventType::NewMarket => {
                info!("📊 新市场上线");
                self.print_market_info(&event.market);
            }
            EventType::PriceChange => {
                info!("💹 价格变化");
                self.print_price_change(&event.market);
            }
            EventType::VolumeUpdate => {
                debug!("📈 成交量更新");
            }
            EventType::MarketClosed => {
                info!("🔒 市场关闭: {}", event.market.question);
            }
        }

        // 保存到数据库
        if let Some(db) = &self.database {
            tokio::spawn({
                let db = db.clone();
                let event = event.clone();
                async move {
                    // 用户要求：只存储 end=False (未关闭) 的市场
                    if event.market.closed != Some(true) {
                        if let Err(e) = db.save_market(&event.market).await {
                            error!("保存市场数据失败: {}", e);
                        }
                    }
                }
            });
        }
    }

    /// 扫描所有市场并存储到数据库（流式处理）
    pub async fn scan_all_markets(&self, batch_size: u32) -> Result<()> {
        info!("开始流式扫描所有市场...");

        let db = self.database.clone();

        // 使用流式处理，逐批保存数据
        let total_count = self
            .client
            .get_all_markets_stream(batch_size, |markets| {
                let db = db.clone();
                async move {
                    if let Some(db) = db {
                        info!("正在保存 {} 个市场到数据库...", markets.len());

                        let mut markets_to_save = Vec::new();
                        for market in markets {
                            // 用户要求：只存储 end=False (未关闭) 的市场
                            if market.closed == Some(true) {
                                continue;
                            }
                            markets_to_save.push(market);
                        }

                        if !markets_to_save.is_empty() {
                            if let Err(e) = db.save_markets(markets_to_save).await {
                                error!("批量保存市场失败: {}", e);
                            } else {
                                debug!("已批量保存市场");
                            }
                        }
                    } else {
                        warn!("未配置数据库，跳过保存");
                    }
                    Ok(())
                }
            })
            .await?;

        info!("扫描完成！共处理 {} 个市场", total_count);
        Ok(())
    }

    /// 打印市场信息
    fn print_market_info(&self, market: &Market) {
        println!("\n═══════════════════════════════════════════");
        println!("问题: {}", market.question);
        if let Some(desc) = &market.description {
            println!("描述: {}", desc);
        }
        println!("───────────────────────────────────────────");
        if let Ok(outcomes) = serde_json::from_str::<Vec<String>>(&market.outcomes) {
            if let Some(prices_str) = &market.outcome_prices {
                if let Ok(outcome_prices) = serde_json::from_str::<Vec<String>>(prices_str) {
                    for (i, outcome) in outcomes.iter().enumerate() {
                        if i < outcome_prices.len() {
                            println!("  {} - 价格: {}", outcome, outcome_prices[i]);
                        }
                    }
                }
            } else {
                for outcome in outcomes {
                    println!("  {} - 价格: N/A", outcome);
                }
            }
        }
        println!("───────────────────────────────────────────");
        if let Some(volume) = &market.volume {
            println!("成交量: ${}", volume);
        }
        if let Some(liquidity) = &market.liquidity {
            println!("流动性: ${}", liquidity);
        }
        if let Some(end_date) = &market.end_date {
            println!("结束日期: {}", end_date);
        }
        println!("═══════════════════════════════════════════\n");
    }

    /// 打印价格变化
    fn print_price_change(&self, market: &Market) {
        println!("\n🔔 {} - 价格更新:", market.question);
        if let Ok(outcomes) = serde_json::from_str::<Vec<String>>(&market.outcomes) {
            if let Some(prices_str) = &market.outcome_prices {
                if let Ok(outcome_prices) = serde_json::from_str::<Vec<f64>>(prices_str) {
                    for (i, outcome) in outcomes.iter().enumerate() {
                        if i < outcome_prices.len() {
                            println!("  {} → {}", outcome, outcome_prices[i]);
                        }
                    }
                }
            }
        }
        println!();
    }
}
