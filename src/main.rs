mod api;
mod scanner;
mod types;
mod error;
mod database;

use anyhow::Result;
use log::{info, error};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();
    
    info!("启动 Polymarket 扫描器...");
    
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 创建 API 客户端
    let client = api::PolymarketClient::new()?;
    
    // 创建 Redis 连接
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    let db = database::Database::new(&redis_url).await?;
    db.init_schema().await?;
    
    info!("数据库初始化完成");
    
    // 创建扫描器
    let scanner = scanner::MarketScanner::with_database(client, Arc::new(db));
    
    // 检查是否需要先扫描所有市场
    if std::env::var("SCAN_ALL_FIRST").unwrap_or_default() == "true" {
        info!("首次运行：扫描所有市场...");
        scanner.scan_all_markets(100).await?;
        info!("所有市场扫描完成");
    }
    
    // 开始持续扫描
    match scanner.start_scanning(Duration::from_secs(10)).await {
        Ok(_) => info!("扫描器正常关闭"),
        Err(e) => error!("扫描器错误: {}", e),
    }
    
    Ok(())
}

