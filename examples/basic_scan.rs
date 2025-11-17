use polymarket_scanner::api::PolymarketClient;
use polymarket_scanner::scanner::MarketScanner;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("启动基础扫描示例...\n");
    
    // 创建 API 客户端
    let client = PolymarketClient::new()?;
    
    // 创建扫描器
    let scanner = MarketScanner::new(client);
    
    // 每 15 秒扫描一次
    scanner.start_scanning(Duration::from_secs(15)).await?;
    
    Ok(())
}

