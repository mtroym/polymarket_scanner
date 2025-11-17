use crate::error::{Result, ScannerError};
use crate::types::{Market, PriceHistory};
use log::{info, debug, warn};
use reqwest::Client;
use serde_json::Value;

const GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com";
#[allow(dead_code)]
const CLOB_API_BASE: &str = "https://clob.polymarket.com";

pub struct PolymarketClient {
    client: Client,
}

impl PolymarketClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        Ok(Self { client })
    }
    
    /// 获取活跃市场列表
    pub async fn get_markets(&self, limit: Option<u32>) -> Result<Vec<Market>> {
        let limit = limit.unwrap_or(100);
        let url = format!("{}/markets", GAMMA_API_BASE);
        
        debug!("请求市场列表: {}", url);
        
        let response = self.client
            .get(&url)
            .query(&[("limit", limit.to_string()), ("active", "true".to_string())])
            .send()
            .await?;
        
        let markets: Vec<Market> = if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("API 请求失败 [{}]: {}", status, text);
            return Err(ScannerError::InvalidResponse(format!("HTTP {}: {}", status, text)));
        } else {
            let markets = response.json().await.unwrap_or_else(|e| {
                warn!("JSON 解析错误: {}", e);
                Vec::new()
            });
            debug!("response: {:?}", markets);
            markets
        };
        
        debug!("成功获取 {} 个市场", markets.len());
        
        Ok(markets)
    }
    
    /// 获取市场列表（支持分页）
    pub async fn get_markets_paginated(&self, limit: u32, offset: u32) -> Result<Vec<Market>> {
        let url = format!("{}/markets", GAMMA_API_BASE);
        
        debug!("请求市场列表（分页）: limit={}, offset={}", limit, offset);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("limit", limit.to_string()),
                ("offset", offset.to_string()),
                ("active", "true".to_string())
            ])
            .send()
            .await?;
        
        let markets: Vec<Market> = if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("API 请求失败 [{}]: {}", status, text);
            return Err(ScannerError::InvalidResponse(format!("HTTP {}: {}", status, text)));
        } else {
            response.json().await.unwrap_or_else(|e| {
                warn!("JSON 解析错误: {}", e);
                Vec::new()
            })
        };
        
        debug!("成功获取 {} 个市场", markets.len());
        Ok(markets)
    }
    
    /// 获取所有市场（流式处理，使用回调函数）
    /// 
    /// 此方法使用流式处理，逐批获取和处理市场数据，避免内存快速增长
    /// 
    /// # 参数
    /// - `batch_size`: 每批获取的市场数量
    /// - `callback`: 处理每批市场数据的回调函数
    /// 
    /// # 示例
    /// ```ignore
    /// client.get_all_markets_stream(100, |batch| async move {
    ///     // 处理每批数据，处理完后内存会被释放
    ///     for market in batch {
    ///         db.save_market(&market).await?;
    ///     }
    ///     Ok(())
    /// }).await?;
    /// ```
    pub async fn get_all_markets_stream<F, Fut>(
        &self,
        batch_size: u32,
        mut callback: F,
    ) -> Result<usize>
    where
        F: FnMut(Vec<Market>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!("开始流式获取所有市场数据，批次大小: {}", batch_size);
        let mut offset = 0;
        let mut total_count = 0;
        
        loop {
            let markets = self.get_markets_paginated(batch_size, offset).await?;
            let count = markets.len();
            
            // if count == 0 {
            //     break;
            // }
            
            info!("获取到第 {} - {} 个市场", offset + 1, offset + count as u32);
            total_count += count;
            
            // 调用回调函数处理当前批次，处理完后这批数据就可以被释放
            callback(markets).await?;
            
            if count < batch_size as usize {
                break; // 最后一页
            }
            
            offset += batch_size;
            
            // 添加延迟避免触发速率限制
            // tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        info!("总共获取 {} 个市场", total_count);
        Ok(total_count)
    }
    
    /// 获取所有市场（一次性加载到内存）
    /// 
    /// ⚠️ 警告：此方法会将所有市场加载到内存中，对于大量数据建议使用 `get_all_markets_stream`
    #[allow(dead_code)]
    pub async fn get_all_markets(&self, batch_size: u32) -> Result<Vec<Market>> {
        info!("开始获取所有市场数据，批次大小: {}", batch_size);
        let mut all_markets = Vec::new();
        let mut offset = 0;
        
        loop {
            let markets = self.get_markets_paginated(batch_size, offset).await?;
            let count = markets.len();
            
            if count == 0 {
                break;
            }
            
            info!("获取到第 {} - {} 个市场", offset + 1, offset + count as u32);
            all_markets.extend(markets);
            
            if count < batch_size as usize {
                break; // 最后一页
            }
            
            offset += batch_size;
            
            // 添加延迟避免触发速率限制
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        info!("总共获取 {} 个市场", all_markets.len());
        Ok(all_markets)
    }
    
    /// 获取市场详情
    #[allow(dead_code)]
    pub async fn get_market(&self, condition_id: &str) -> Result<Market> {
        let url = format!("{}/markets/{}", CLOB_API_BASE, condition_id);
        
        debug!("请求市场详情: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(ScannerError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }
        
        let market: Market = response.json().await?;
        Ok(market)
    }
    
    /// 获取价格历史
    #[allow(dead_code)]
    pub async fn get_price_history(
        &self,
        market_id: &str,
        start_ts: Option<i64>,
        end_ts: Option<i64>,
    ) -> Result<Vec<PriceHistory>> {
        let url = format!("{}/prices-history", GAMMA_API_BASE);
        
        let mut query_params = vec![("market", market_id.to_string())];
        
        if let Some(start) = start_ts {
            query_params.push(("startTs", start.to_string()));
        }
        if let Some(end) = end_ts {
            query_params.push(("endTs", end.to_string()));
        }
        
        debug!("请求价格历史: {}", url);
        
        let response = self.client
            .get(&url)
            .query(&query_params)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(ScannerError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }
        
        let history: Vec<PriceHistory> = response.json().await?;
        Ok(history)
    }
    
    /// 获取市场统计信息
    #[allow(dead_code)]
    pub async fn get_market_stats(&self, condition_id: &str) -> Result<Value> {
        let url = format!("{}/markets/{}/stats", CLOB_API_BASE, condition_id);
        
        debug!("请求市场统计: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(ScannerError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }
        
        let stats: Value = response.json().await?;
        Ok(stats)
    }
}

