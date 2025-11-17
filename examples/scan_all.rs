use polymarket_scanner::{PolymarketClient, MarketScanner, Database};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("扫描所有 Polymarket 市场数据...\n");
    
    // 创建客户端
    let client = PolymarketClient::new()?;
    
    // 创建 Redis 连接
    let db = Database::new("redis://127.0.0.1:6379").await?;
    db.init_schema().await?;
    
    println!("数据库初始化完成\n");
    
    // 创建扫描器
    let scanner = MarketScanner::with_database(client, Arc::new(db));
    
    // 扫描所有市场（每批100个）
    scanner.scan_all_markets(500).await?;
    
    println!("\n所有市场数据扫描完成！");
    
    Ok(())
}

