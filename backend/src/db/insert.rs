
use time::{Date, OffsetDateTime};

use anyhow::Result;
use sqlx::{PgPool, QueryBuilder};
use sqlx::types::BigDecimal;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use chrono::Datelike;
use crate::exchanges::shared::types::{
    NormalizedFundingRate, NormalizedMarket, NormalizedMarketStats,NormalizedKline, NormalizedTrade,CexMarket
};


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


pub async fn insert_cex_klines(
    pool: &PgPool,
    klines: Vec<(i32, NormalizedKline)>,
    interval: &str,
) -> Result<()> {
    if klines.is_empty() { return Ok(()); }

    // Table, PK column, and PG type for the 2nd UNNEST arg
    let (table_name, pk_col, pg_type_str) = match interval {
        "1d" => ("klines_daily", "date", "date"),
        "1h" => ("klines_hourly", "time", "timestamptz"),
        _ => anyhow::bail!("Unsupported kline interval: {}", interval),
    };

    let mut market_ids: Vec<i32>                  = Vec::with_capacity(klines.len());
    let mut date_vals:   Vec<time::Date>          = Vec::with_capacity(klines.len());         // for 1d
    let mut time_vals:   Vec<OffsetDateTime>      = Vec::with_capacity(klines.len());         // for 1h
    let mut opens:       Vec<BigDecimal>          = Vec::with_capacity(klines.len());
    let mut highs:       Vec<BigDecimal>          = Vec::with_capacity(klines.len());
    let mut lows:        Vec<BigDecimal>          = Vec::with_capacity(klines.len());
    let mut closes:      Vec<BigDecimal>          = Vec::with_capacity(klines.len());
    let mut volumes:     Vec<BigDecimal>          = Vec::with_capacity(klines.len());

    for (market_id, k) in klines {
        market_ids.push(market_id);
        opens.push(BigDecimal::from_str(&k.open.to_string())?);
        highs.push(BigDecimal::from_str(&k.high.to_string())?);
        lows.push(BigDecimal::from_str(&k.low.to_string())?);
        closes.push(BigDecimal::from_str(&k.close.to_string())?);
        volumes.push(BigDecimal::from_str(&k.volume.to_string())?);

        match interval {
            "1d" => {
                // Convert chrono -> time::Date (Month expects u8; day expects u8)
                let month = time::Month::try_from(k.open_time.month() as u8)?;
                let day: u8 = k.open_time.day() as u8;
                let date = time::Date::from_calendar_date(k.open_time.year(), month, day)?;
                date_vals.push(date);
            }
            "1h" => {
                let ts = OffsetDateTime::from_unix_timestamp(k.open_time.timestamp())?;
                time_vals.push(ts);
            }
            _ => unreachable!(),
        }
    }

    // Build query that inserts with the correct PK column and PG array type
    let query_str = format!(
        r#"
        INSERT INTO {table} (market_id, {pk}, "open", high, low, "close", volume)
        SELECT u.market_id, u.{pk}, u.open, u.high, u.low, u.close, u.volume
        FROM UNNEST($1::int[], $2::{pg}[], $3::numeric[], $4::numeric[], $5::numeric[], $6::numeric[], $7::numeric[])
             AS u(market_id, {pk}, "open", high, low, "close", volume)
        ON CONFLICT (market_id, {pk}) DO NOTHING
        "#,
        table = table_name,
        pk    = pk_col,
        pg    = pg_type_str,
    );

    let mut q = sqlx::query(&query_str).bind(&market_ids);

    // Bind the correct temporal array based on interval
    q = match interval {
        "1d" => q.bind(&date_vals),
        "1h" => q.bind(&time_vals),
        _ => unreachable!(),
    };

    q.bind(&opens)
        .bind(&highs)
        .bind(&lows)
        .bind(&closes)
        .bind(&volumes)
        .execute(pool)
        .await?;

    Ok(())
}


pub async fn insert_cex_trades(pool: &PgPool, trades: Vec<(i32, NormalizedTrade)>) -> Result<()> {
    if trades.is_empty() { return Ok(()); }

    let mut market_ids = Vec::with_capacity(trades.len());
    let mut trade_ids = Vec::with_capacity(trades.len());
    let mut trade_times_for_sqlx = Vec::with_capacity(trades.len());
    let mut sides = Vec::with_capacity(trades.len());
    let mut prices = Vec::with_capacity(trades.len());
    let mut qtys = Vec::with_capacity(trades.len());
    let mut quote_qtys = Vec::with_capacity(trades.len());

    for (market_id, t) in trades {
        market_ids.push(market_id);
        trade_ids.push(t.trade_id);
        trade_times_for_sqlx.push(OffsetDateTime::from_unix_timestamp(t.trade_time.timestamp())?);
        sides.push(t.side.to_lowercase());
        prices.push(BigDecimal::from_str(&t.price.to_string())?);
        qtys.push(BigDecimal::from_str(&t.qty.to_string())?);
        quote_qtys.push(BigDecimal::from_str(&t.quote_qty.to_string())?);
    }
    
    sqlx::query!(r#"
        INSERT INTO trades (market_id, trade_id, trade_time, side, price, qty, quote_qty)
        SELECT u.market_id, u.trade_id, u.trade_time, u.side, u.price, u.qty, u.quote_qty
        FROM UNNEST($1::int[], $2::text[], $3::timestamptz[], $4::text[], $5::numeric[], $6::numeric[], $7::numeric[])
        AS u(market_id, trade_id, trade_time, side, price, qty, quote_qty)
        ON CONFLICT (market_id, trade_id) DO NOTHING
        "#,
        &market_ids,
        &trade_ids,
        &trade_times_for_sqlx as &[OffsetDateTime],
        &sides,
        &prices as &[BigDecimal],
        &qtys as &[BigDecimal],
        &quote_qtys as &[BigDecimal]
    ).execute(pool).await?;

    Ok(())
}







pub async fn upsert_cex_exchange(pool: &PgPool, name: &str) -> Result<i32> {
    let rec = sqlx::query!(
        r#"
        INSERT INTO cex_exchanges (name, is_active)
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


pub async fn upsert_cex_markets(pool: &PgPool, exchange_id: i32, markets: &[CexMarket]) -> Result<()> {
    if markets.is_empty() {
        return Ok(());
    }

    let mut symbols        = Vec::with_capacity(markets.len());
    let mut market_symbols = Vec::with_capacity(markets.len());
    let mut base_assets    = Vec::with_capacity(markets.len());
    let mut quote_assets   = Vec::with_capacity(markets.len());
    let mut market_types   = Vec::with_capacity(markets.len());
    let mut actives        = Vec::with_capacity(markets.len());

    for m in markets {
        symbols.push(m.symbol.clone());            // base only, keep case
        market_symbols.push(m.market_symbol.clone());
        base_assets.push(m.base_currency.clone());
        quote_assets.push(m.quote_currency.clone());
        market_types.push(m.market_type.clone());  // "spot" or "perps"
        actives.push(m.is_active);
    }

    sqlx::query!(
        r#"
        INSERT INTO cex_markets (
            exchange_id, symbol, market_symbol, base_asset, quote_asset, market_type, is_active
        )
        SELECT
            $1,
            u.symbol,
            u.market_symbol,
            u.base_asset,
            u.quote_asset,
            u.market_type,
            u.is_active
        FROM UNNEST(
            $2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::bool[]
        ) AS u(symbol, market_symbol, base_asset, quote_asset, market_type, is_active)
        ON CONFLICT (exchange_id, market_symbol, market_type) DO UPDATE
        SET
            symbol      = EXCLUDED.symbol,
            base_asset  = EXCLUDED.base_asset,
            quote_asset = EXCLUDED.quote_asset,
            is_active   = EXCLUDED.is_active,
            updated_at  = NOW()
        "#,
        exchange_id,
        &symbols,
        &market_symbols,
        &base_assets,
        &quote_assets,
        &market_types,
        &actives
    )
    .execute(pool)
    .await?;

    Ok(())
}