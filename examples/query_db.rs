use polymarket_scanner::{JsonDatabase, Storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();

    println!("查询 Polymarket Redis 数据库...\n");

    // 连接 Redis
    let db = JsonDatabase::new("./data");
    db.init().await?;
    // 获取统计信息
    let market_count = db.get_all_market_ids().await?.len();

    println!("═══════════════════════════════════════════");
    println!("数据库统计信息:");
    println!("───────────────────────────────────────────");
    println!("市场总数: {}", market_count);
    println!("═══════════════════════════════════════════\n");

    Ok(())
}
