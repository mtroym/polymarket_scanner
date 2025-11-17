use polymarket_scanner::api::PolymarketClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("获取 Polymarket 市场列表...\n");
    
    // 创建客户端
    let client = PolymarketClient::new()?;
    
    // 获取前 20 个活跃市场
    let markets = client.get_markets(Some(20)).await?;
    
    println!("找到 {} 个活跃市场:\n", markets.len());
    
    for (i, market) in markets.iter().enumerate() {
        println!("{}. {}", i + 1, market.question);
        println!("   结果选项: {:?}", market.outcomes);
        println!("   当前价格: {:?}", market.outcome_prices);
        
        if let Some(volume) = &market.volume {
            println!("   成交量: ${}", volume);
        }
        
        if let Some(liquidity) = &market.liquidity {
            println!("   流动性: ${}", liquidity);
        }
        
        println!();
    }
    
    Ok(())
}

