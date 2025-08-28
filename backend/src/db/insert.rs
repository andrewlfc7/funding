

use anyhow::Result;
use sqlx::{PgPool, QueryBuilder};
use sqlx::types::BigDecimal;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use time::OffsetDateTime;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::exchanges::shared::types::{
    NormalizedFundingRate, NormalizedMarket, NormalizedMarketStats,
};

/// Upsert the exchange row and return its id.
pub async fn upsert_exchange(pool: &PgPool, name: &str) -> Result<i32> {
    let rec = sqlx::query!(
        r#"
        INSERT INTO exchanges(name, is_active)
        VALUES ($1, TRUE)
        ON CONFLICT (name)
            DO UPDATE SET updated_at = NOW(), is_active = TRUE
        RETURNING id
        "#,
        name
    )
    .fetch_one(pool)
    .await?;
    Ok(rec.id)
}

pub async fn upsert_markets(
    pool: &PgPool,
    exchange_id: i32,
    markets: &[NormalizedMarket],
) -> Result<()> {
    if markets.is_empty() {
        return Ok(());
    }

    let mut token_set: HashSet<&str> = HashSet::new();
    for m in markets {
        token_set.insert(m.symbol.as_str());
    }
    let tokens: Vec<&str> = token_set.into_iter().collect();

    {
        let mut qb = QueryBuilder::new("INSERT INTO tokens (symbol) ");
        qb.push_values(tokens.iter(), |mut b, sym| {
            b.push_bind(*sym);
        });
        qb.push(" ON CONFLICT(symbol) DO NOTHING");
        qb.build().execute(pool).await?;
    }

    let mut qb_ids = QueryBuilder::new("SELECT id, symbol FROM tokens WHERE symbol IN (");
    let mut sep = qb_ids.separated(", ");
    for sym in &tokens {
        sep.push_bind(*sym);
    }
    sep.push_unseparated(")");
    let rows = qb_ids
        .build_query_as::<(i32, String)>()
        .fetch_all(pool)
        .await?;

    let mut token_id_by_symbol: HashMap<String, i32> = HashMap::with_capacity(rows.len());
    for (id, sym) in rows {
        token_id_by_symbol.insert(sym, id);
    }

    {
        let mut qb = QueryBuilder::new(
            r#"
            INSERT INTO markets (exchange_id, token_id, market_symbol, is_active)
            "#,
        );
        qb.push_values(markets.iter(), |mut b, m| {
            let token_id = *token_id_by_symbol
                .get(&m.symbol)
                .expect("token id should exist after bulk insert");
            b.push_bind(exchange_id)
                .push_bind(token_id)
                .push_bind(&m.market_symbol)
                .push_bind(m.is_active);
        });
        qb.push(
            r#"
            ON CONFLICT(exchange_id, market_symbol)
            DO UPDATE SET is_active = EXCLUDED.is_active, updated_at = NOW()
            "#,
        );
        qb.build().execute(pool).await?;
    }

    Ok(())
}


pub async fn insert_funding_rates(
    pool: &PgPool,
    exchange_id: i32,
    rows: &[(i32, &NormalizedFundingRate)],
) -> Result<()> {
    if rows.is_empty() { return Ok(()); }

    // Make chunk size tunable
    let chunk_rows: usize = std::env::var("SYNC_DB_CHUNK")
        .ok().and_then(|v| v.parse().ok())
        .unwrap_or(20_000); 

    let mut tx = pool.begin().await?;
    sqlx::query!("SET LOCAL synchronous_commit = 'off'")
        .execute(&mut *tx).await?;

    let mut start = 0;
    while start < rows.len() {
        let end = (start + chunk_rows).min(rows.len());
        let slice = &rows[start..end];

        let mut market_ids = Vec::with_capacity(slice.len());
        let mut rates_bd   = Vec::with_capacity(slice.len());
        let mut ts_time    = Vec::with_capacity(slice.len());

        for (mid, r) in slice {
            market_ids.push(*mid);
            rates_bd.push(sqlx::types::BigDecimal::from_str(&r.rate.to_string()).unwrap());
            let ts = time::OffsetDateTime::from_unix_timestamp(r.timestamp.timestamp()).unwrap();
            ts_time.push(ts);
        }

        sqlx::query!(
            r#"
            INSERT INTO funding_rates (exchange_id, market_id, rate, timestamp)
            SELECT $1::int4, u.market_id, u.rate, u.ts
            FROM UNNEST($2::int4[], $3::numeric[], $4::timestamptz[]) AS u(market_id, rate, ts)
            ON CONFLICT (market_id, timestamp) DO NOTHING
            "#,
            exchange_id, &market_ids, &rates_bd, &ts_time
        )
        .execute(&mut *tx)
        .await?;

        start = end;
    }

    tx.commit().await?;
    Ok(())
}


pub async fn insert_market_stats(
    pool: &PgPool,
    rows: &[(i32, &NormalizedMarketStats)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut qb = QueryBuilder::new(
        r#"
        INSERT INTO market_stats (market_id, open_interest, volume_24h, timestamp)
        "#,
    );

    qb.push_values(rows.iter(), |mut b, (market_id, stat)| {
        let oi = stat
            .open_interest
            .as_ref()
            .map(|d| BigDecimal::from_str(&d.to_string()).expect("decimal oi"));
        let vol = stat
            .volume_24h
            .as_ref()
            .map(|d| BigDecimal::from_str(&d.to_string()).expect("decimal vol"));
        let ts = OffsetDateTime::from_unix_timestamp(stat.timestamp.timestamp())
            .expect("valid ts");

        b.push_bind(*market_id)
            .push_bind(oi)
            .push_bind(vol)
            .push_bind(ts);
    });

    qb.push(" ON CONFLICT (market_id, timestamp) DO NOTHING");
    qb.build().execute(pool).await?;
    Ok(())
}



pub async fn insert_market_stats_by_symbol(
    pool: &PgPool,
    exchange_id: i32,
    rows: &[(String, &NormalizedMarketStats)],
) -> anyhow::Result<()> {
    if rows.is_empty() { return Ok(()); }

    let mut symbols: Vec<String>              = Vec::with_capacity(rows.len());
    let mut oi:      Vec<Option<BigDecimal>>  = Vec::with_capacity(rows.len());
    let mut vol:     Vec<Option<BigDecimal>>  = Vec::with_capacity(rows.len());
    let mut ts:      Vec<OffsetDateTime>      = Vec::with_capacity(rows.len());

    for (sym, stat) in rows {
        symbols.push(sym.clone());
        oi.push(stat.open_interest.as_ref().map(|d| BigDecimal::from_str(&d.to_string()).unwrap()));
        vol.push(stat.volume_24h.as_ref().map(|d| BigDecimal::from_str(&d.to_string()).unwrap()));
        ts.push(OffsetDateTime::from_unix_timestamp(stat.timestamp.timestamp()).unwrap());
    }

    sqlx::query(
        r#"
        INSERT INTO market_stats (market_id, open_interest, volume_24h, timestamp)
        SELECT m.id, u.oi, u.vol, u.ts
        FROM UNNEST($1::text[], $2::numeric[], $3::numeric[], $4::timestamptz[]) AS u(market_symbol, oi, vol, ts)
        JOIN markets m
          ON m.exchange_id = $5
         AND m.market_symbol = u.market_symbol
        ON CONFLICT (market_id, timestamp) DO NOTHING
        "#
    )
    .bind(&symbols)   
    .bind(&oi)        
    .bind(&vol)      
    .bind(&ts)        
    .bind(exchange_id) 
    .execute(pool)
    .await?;

    Ok(())
}
