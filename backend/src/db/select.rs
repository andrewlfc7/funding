use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;

/// Load {market_symbol -> market_id} for a given exchange.
pub async fn load_market_ids(pool: &PgPool, exchange_id: i32) -> Result<HashMap<String, i32>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, market_symbol
        FROM markets
        WHERE exchange_id = $1
        "#,
        exchange_id
    )
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::with_capacity(rows.len());
    for r in rows {
        map.insert(r.market_symbol, r.id);
    }
    Ok(map)
}
