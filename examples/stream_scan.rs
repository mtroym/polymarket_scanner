use polymarket_scanner::{PolymarketClient, Database};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("使用流式处理扫描所有 Polymarket 市场数据...\n");
    
    // 创建客户端
    let client = PolymarketClient::new()?;
    
    // 创建 Redis 连接
    let db = Arc::new(Database::new("redis://127.0.0.1:6379").await?);
    db.init_schema().await?;
    
    println!("Redis 连接成功\n");
    
    // 计数器
    let total_processed = Arc::new(AtomicUsize::new(0));
    let total_saved = Arc::new(AtomicUsize::new(0));
    
    // 使用流式处理，每批处理 100 个市场
    let result = client.get_all_markets_stream(1000, |markets| {
        let db = Arc::clone(&db);
        let total_processed = Arc::clone(&total_processed);
        let total_saved = Arc::clone(&total_saved);
        
        async move {
            let batch_size = markets.len();
            println!("📦 处理批次: {} 个市场", batch_size);
            
            let mut saved_count = 0;
            for market in markets {
                match db.save_market(&market).await {
                    Ok(_) => {
                        saved_count += 1;
                        println!("  ✓ {}", market.question);
                    }
                    Err(e) => {
                        eprintln!("  ✗ 保存失败: {} - {}", market.question, e);
                    }
                }
            }
            
            total_processed.fetch_add(batch_size, Ordering::SeqCst);
            total_saved.fetch_add(saved_count, Ordering::SeqCst);
            
            println!("  批次保存完成: {}/{}\n", saved_count, batch_size);
            
            Ok(())
        }
    }).await?;
    
    println!("═══════════════════════════════════════════");
    println!("扫描完成！");
    println!("───────────────────────────────────────────");
    println!("总市场数: {}", result);
    println!("已处理: {}", total_processed.load(Ordering::SeqCst));
    println!("已保存: {}", total_saved.load(Ordering::SeqCst));
    println!("═══════════════════════════════════════════");
    
    Ok(())
}

