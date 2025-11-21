use polymarket_scanner::json_db::JsonDatabase;
use polymarket_scanner::storage::Storage;
use polymarket_scanner::types::Market;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = PathBuf::from("test_data");
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).await?;
    }
    fs::create_dir_all(&test_dir).await?;

    let db = JsonDatabase::new(&test_dir);
    db.init().await?;

    let mut markets = Vec::new();
    for i in 0..10 {
        markets.push(Market {
            condition_id: format!("condition_{}", i),
            question_id: Some(format!("question_{}", i)),
            question: format!("Question {}", i),
            description: None,
            market_slug: None,
            outcomes: "[\"Yes\", \"No\"]".to_string(),
            outcome_prices: Some("[0.5, 0.5]".to_string()),
            volume: Some("1000".to_string()),
            liquidity: None,
            end_date: None,
            active: Some(true),
            closed: Some(false),
        });
    }

    println!("Saving {} markets...", markets.len());
    db.save_markets(markets.clone()).await?;

    let count = db.get_market_count().await?;
    println!("Market count: {}", count);

    assert_eq!(count, 10);

    let market = db.get_market("condition_0").await?;
    assert!(market.is_some());
    assert_eq!(market.unwrap().question, "Question 0");

    // Verify file content
    let content = fs::read_to_string(test_dir.join("markets.json")).await?;
    assert!(content.contains("Question 0"));
    assert!(content.contains("Question 9"));

    println!("Verification successful!");

    fs::remove_dir_all(&test_dir).await?;

    Ok(())
}
