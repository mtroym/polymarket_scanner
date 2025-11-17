use polymarket_scanner::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("查询 Polymarket Redis 数据库...\n");
    
    // 连接 Redis
    let db = Database::new("redis://127.0.0.1:6379").await?;
    
    // 获取统计信息
    let market_count = db.get_market_count().await?;
    let event_count = db.get_event_count().await?;
    
    println!("═══════════════════════════════════════════");
    println!("数据库统计信息:");
    println!("───────────────────────────────────────────");
    println!("市场总数: {}", market_count);
    println!("事件总数: {}", event_count);
    println!("═══════════════════════════════════════════\n");
    
    // 获取最近的事件
    println!("最近的 10 个市场事件:\n");
    let recent_events = db.get_recent_events(10).await?;
    
    for (i, (event_type, question, prices, timestamp)) in recent_events.iter().enumerate() {
        println!("{}. [{}] {}", i + 1, event_type, question);
        println!("   价格: {}", prices);
        println!("   时间: {}", timestamp.format("%Y-%m-%d %H:%M:%S"));
        println!();
    }
    
    Ok(())
}

