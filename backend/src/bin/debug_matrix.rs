use backend::db::{clickhouse, migrations};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let client = migrations::create_pool().await?;
    let rows = clickhouse::funding_matrix_rows(&client).await?;
    println!("rows={}", rows.len());
    if let Some(r) = rows.first() {
        println!(
            "sample token={} exchange={} market={}",
            r.token, r.exchange, r.market_symbol
        );
    }
    Ok(())
}
