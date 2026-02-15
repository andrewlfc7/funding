use ::clickhouse::{Client, Row};
use anyhow::{Context, Result, anyhow, bail};
use chrono::{Datelike, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::exchanges::shared::types::{
    NormalizedFundingRate, NormalizedKline, NormalizedMarketStats, NormalizedTrade,
};

#[derive(Debug, Clone)]
pub struct MarketUpsert {
    pub id: i32,
    pub exchange_id: i32,
    pub token_id: i32,
    pub market_symbol: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct CexMarketUpsert {
    pub id: i32,
    pub exchange_id: i32,
    pub symbol: String,
    pub market_symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub market_type: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct FundingMatrixRow {
    pub token: String,
    pub exchange: String,
    pub market_symbol: String,
    pub funding_rate: Option<f64>,
    pub funding_ts_ms: Option<i64>,
    pub open_interest: Option<f64>,
    pub volume_24h: Option<f64>,
    pub stats_ts_ms: Option<i64>,
}

#[derive(Debug, Row, Serialize)]
struct ExchangeRow {
    id: i32,
    name: String,
    is_active: u8,
    updated_at_ms: i64,
}

#[derive(Debug, Row, Serialize)]
struct TokenRow {
    id: i32,
    symbol: String,
}

#[derive(Debug, Row, Serialize)]
struct MarketRow {
    id: i32,
    exchange_id: i32,
    token_id: i32,
    market_symbol: String,
    is_active: u8,
    updated_at_ms: i64,
}

#[derive(Debug, Row, Serialize)]
struct CexExchangeRow {
    id: i32,
    name: String,
    is_active: u8,
    updated_at_ms: i64,
}

#[derive(Debug, Row, Serialize)]
struct CexMarketRow {
    id: i32,
    exchange_id: i32,
    symbol: String,
    market_symbol: String,
    base_asset: String,
    quote_asset: String,
    market_type: String,
    is_active: u8,
    updated_at_ms: i64,
}

#[derive(Debug, Row, Serialize)]
struct FundingRateRow {
    exchange_id: i32,
    market_id: i32,
    rate: f64,
    ts_ms: i64,
}

#[derive(Debug, Row, Serialize)]
struct MarketStatRow {
    market_id: i32,
    open_interest: Option<f64>,
    volume_24h: Option<f64>,
    ts_ms: i64,
}

#[derive(Debug, Row, Serialize)]
struct KlineDailyRow {
    market_id: i32,
    date_key: i32,
    open_px: f64,
    high_px: f64,
    low_px: f64,
    close_px: f64,
    volume: f64,
}

#[derive(Debug, Row, Serialize)]
struct KlineHourlyRow {
    market_id: i32,
    time_ms: i64,
    open_px: f64,
    high_px: f64,
    low_px: f64,
    close_px: f64,
    volume: f64,
}

#[derive(Debug, Row, Serialize)]
struct TradeRow {
    market_id: i32,
    trade_id: String,
    trade_time_ms: i64,
    side: String,
    price: f64,
    qty: f64,
    quote_qty: f64,
}

#[derive(Debug, Row, Deserialize)]
struct IdNameRow {
    id: i32,
    name: String,
}

#[derive(Debug, Row, Deserialize)]
struct IdSymbolRow {
    id: i32,
    market_symbol: String,
}

#[derive(Debug, Row, Deserialize)]
struct LastTsRow {
    last_ts_ms: Option<i64>,
}

#[derive(Debug, Row, Deserialize)]
struct LastDateKeyRow {
    last_date_key: Option<i32>,
}

#[derive(Debug, Row, Deserialize)]
struct FundingMatrixWireRow {
    token: String,
    exchange: String,
    market_symbol: String,
    funding_rate: Option<f64>,
    funding_ts_ms: Option<i64>,
    open_interest: Option<f64>,
    volume_24h: Option<f64>,
    stats_ts_ms: Option<i64>,
}

#[derive(Debug, Row, Deserialize)]
struct CountRow {
    cnt: u64,
}

pub async fn create_client_from_env() -> Result<Client> {
    let url =
        std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".to_string());
    let database = std::env::var("CLICKHOUSE_DATABASE").unwrap_or_else(|_| "crypto_db".to_string());
    validate_identifier(&database)?;

    let user = std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string());
    let password = std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_default();

    let mut admin = Client::default()
        .with_url(url.clone())
        .with_user(user.clone());
    if !password.is_empty() {
        admin = admin.with_password(password.clone());
    }

    admin
        .query(&format!("CREATE DATABASE IF NOT EXISTS {}", database))
        .execute()
        .await
        .with_context(|| format!("creating ClickHouse database `{database}`"))?;

    let mut client = Client::default()
        .with_url(url)
        .with_database(database)
        .with_user(user);
    if !password.is_empty() {
        client = client.with_password(password);
    }

    ensure_schema(&client).await?;
    Ok(client)
}

async fn ensure_schema(client: &Client) -> Result<()> {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS exchanges (
            id Int32,
            name String,
            is_active UInt8,
            updated_at_ms Int64
        )
        ENGINE = ReplacingMergeTree(updated_at_ms)
        ORDER BY (id)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS tokens (
            id Int32,
            symbol String
        )
        ENGINE = ReplacingMergeTree
        ORDER BY (id)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS markets (
            id Int32,
            exchange_id Int32,
            token_id Int32,
            market_symbol String,
            is_active UInt8,
            updated_at_ms Int64
        )
        ENGINE = ReplacingMergeTree(updated_at_ms)
        ORDER BY (id)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS cex_exchanges (
            id Int32,
            name String,
            is_active UInt8,
            updated_at_ms Int64
        )
        ENGINE = ReplacingMergeTree(updated_at_ms)
        ORDER BY (id)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS cex_markets (
            id Int32,
            exchange_id Int32,
            symbol String,
            market_symbol String,
            base_asset String,
            quote_asset String,
            market_type String,
            is_active UInt8,
            updated_at_ms Int64
        )
        ENGINE = ReplacingMergeTree(updated_at_ms)
        ORDER BY (id)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS funding_rates (
            exchange_id Int32,
            market_id Int32,
            rate Float64,
            ts_ms Int64,
            inserted_at_ms Int64 DEFAULT toUnixTimestamp64Milli(now64(3))
        )
        ENGINE = ReplacingMergeTree(inserted_at_ms)
        ORDER BY (exchange_id, market_id, ts_ms)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS market_stats (
            market_id Int32,
            open_interest Nullable(Float64),
            volume_24h Nullable(Float64),
            ts_ms Int64,
            inserted_at_ms Int64 DEFAULT toUnixTimestamp64Milli(now64(3))
        )
        ENGINE = ReplacingMergeTree(inserted_at_ms)
        ORDER BY (market_id, ts_ms)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS klines_daily (
            market_id Int32,
            date_key Int32,
            open_px Float64,
            high_px Float64,
            low_px Float64,
            close_px Float64,
            volume Float64,
            inserted_at_ms Int64 DEFAULT toUnixTimestamp64Milli(now64(3))
        )
        ENGINE = ReplacingMergeTree(inserted_at_ms)
        ORDER BY (market_id, date_key)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS klines_hourly (
            market_id Int32,
            time_ms Int64,
            open_px Float64,
            high_px Float64,
            low_px Float64,
            close_px Float64,
            volume Float64,
            inserted_at_ms Int64 DEFAULT toUnixTimestamp64Milli(now64(3))
        )
        ENGINE = ReplacingMergeTree(inserted_at_ms)
        ORDER BY (market_id, time_ms)
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS trades (
            market_id Int32,
            trade_id String,
            trade_time_ms Int64,
            side LowCardinality(String),
            price Float64,
            qty Float64,
            quote_qty Float64,
            inserted_at_ms Int64 DEFAULT toUnixTimestamp64Milli(now64(3))
        )
        ENGINE = ReplacingMergeTree(inserted_at_ms)
        ORDER BY (market_id, trade_id)
        "#,
    ];

    for stmt in statements {
        client.query(stmt).execute().await?;
    }
    Ok(())
}

pub fn exchange_id(name: &str) -> i32 {
    stable_id(&["dex_exchange", &name.trim().to_ascii_lowercase()])
}

pub fn token_id(symbol: &str) -> i32 {
    stable_id(&["token", &symbol.trim().to_ascii_uppercase()])
}

pub fn market_id(exchange_id: i32, market_symbol: &str) -> i32 {
    stable_id(&[
        "dex_market",
        &exchange_id.to_string(),
        &market_symbol.trim().to_ascii_uppercase(),
    ])
}

pub fn cex_exchange_id(name: &str) -> i32 {
    stable_id(&["cex_exchange", &name.trim().to_ascii_lowercase()])
}

pub fn cex_market_id(exchange_id: i32, market_symbol: &str, market_type: &str) -> i32 {
    stable_id(&[
        "cex_market",
        &exchange_id.to_string(),
        &market_symbol.trim().to_ascii_uppercase(),
        &market_type.trim().to_ascii_lowercase(),
    ])
}

pub async fn list_active_exchanges(client: &Client) -> Result<Vec<(i32, String)>> {
    let rows = client
        .query(
            r#"
            SELECT id, name
            FROM exchanges FINAL
            WHERE is_active = 1
            ORDER BY name
            "#,
        )
        .fetch_all::<IdNameRow>()
        .await?;

    Ok(rows.into_iter().map(|r| (r.id, r.name)).collect())
}

pub async fn list_active_markets(client: &Client, exchange_id: i32) -> Result<Vec<(i32, String)>> {
    let rows = client
        .query(
            r#"
            SELECT id, market_symbol
            FROM markets FINAL
            WHERE exchange_id = ? AND is_active = 1
            ORDER BY market_symbol
            "#,
        )
        .bind(exchange_id)
        .fetch_all::<IdSymbolRow>()
        .await?;

    Ok(rows.into_iter().map(|r| (r.id, r.market_symbol)).collect())
}

pub async fn latest_funding_ts_ms(client: &Client, market_id: i32) -> Result<Option<i64>> {
    let row = client
        .query(
            r#"
            SELECT maxOrNull(ts_ms) AS last_ts_ms
            FROM funding_rates
            WHERE market_id = ?
            "#,
        )
        .bind(market_id)
        .fetch_one::<LastTsRow>()
        .await?;

    Ok(row.last_ts_ms)
}

pub async fn list_active_cex_markets(
    client: &Client,
    exchange_id: i32,
    market_type: &str,
) -> Result<Vec<(i32, String)>> {
    let market_type = market_type.trim().to_ascii_lowercase();
    let rows = client
        .query(
            r#"
            SELECT id, market_symbol
            FROM cex_markets FINAL
            WHERE exchange_id = ? AND market_type = ? AND is_active = 1
            ORDER BY market_symbol
            "#,
        )
        .bind(exchange_id)
        .bind(market_type)
        .fetch_all::<IdSymbolRow>()
        .await?;

    Ok(rows.into_iter().map(|r| (r.id, r.market_symbol)).collect())
}

pub async fn latest_kline_ts_ms(
    client: &Client,
    market_id: i32,
    interval: &str,
) -> Result<Option<i64>> {
    match interval {
        "1h" => {
            let row = client
                .query(
                    r#"
                    SELECT maxOrNull(time_ms) AS last_ts_ms
                    FROM klines_hourly
                    WHERE market_id = ?
                    "#,
                )
                .bind(market_id)
                .fetch_one::<LastTsRow>()
                .await?;
            Ok(row.last_ts_ms)
        }
        "1d" => {
            let row = client
                .query(
                    r#"
                    SELECT maxOrNull(date_key) AS last_date_key
                    FROM klines_daily
                    WHERE market_id = ?
                    "#,
                )
                .bind(market_id)
                .fetch_one::<LastDateKeyRow>()
                .await?;

            Ok(row.last_date_key.and_then(date_key_to_midnight_ms))
        }
        _ => bail!("Unsupported kline interval: {interval}"),
    }
}

pub async fn latest_trade_ts_ms(client: &Client, market_id: i32) -> Result<Option<i64>> {
    let row = client
        .query(
            r#"
            SELECT maxOrNull(trade_time_ms) AS last_ts_ms
            FROM trades
            WHERE market_id = ?
            "#,
        )
        .bind(market_id)
        .fetch_one::<LastTsRow>()
        .await?;
    Ok(row.last_ts_ms)
}

pub async fn funding_matrix_rows(client: &Client) -> Result<Vec<FundingMatrixRow>> {
    let rows = client
        .query(
            r#"
            SELECT
                t.symbol AS token,
                e.name AS exchange,
                m.market_symbol AS market_symbol,
                CAST(fr.rate, 'Nullable(Float64)') AS funding_rate,
                CAST(fr.last_ts_ms, 'Nullable(Int64)') AS funding_ts_ms,
                CAST(ms.open_interest, 'Nullable(Float64)') AS open_interest,
                CAST(ms.volume_24h, 'Nullable(Float64)') AS volume_24h,
                CAST(ms.last_ts_ms, 'Nullable(Int64)') AS stats_ts_ms
            FROM markets AS m FINAL
            INNER JOIN tokens AS t FINAL ON t.id = m.token_id
            INNER JOIN exchanges AS e FINAL ON e.id = m.exchange_id
            LEFT JOIN (
                SELECT market_id, argMax(rate, ts_ms) AS rate, max(ts_ms) AS last_ts_ms
                FROM funding_rates
                GROUP BY market_id
            ) fr ON fr.market_id = m.id
            LEFT JOIN (
                SELECT
                    market_id,
                    argMax(open_interest, ts_ms) AS open_interest,
                    argMax(volume_24h, ts_ms) AS volume_24h,
                    max(ts_ms) AS last_ts_ms
                FROM market_stats
                GROUP BY market_id
            ) ms ON ms.market_id = m.id
            WHERE m.is_active = 1 AND e.is_active = 1
            ORDER BY token, exchange
            "#,
        )
        .fetch_all::<FundingMatrixWireRow>()
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| FundingMatrixRow {
            token: r.token,
            exchange: r.exchange,
            market_symbol: r.market_symbol,
            funding_rate: r.funding_rate,
            funding_ts_ms: r.funding_ts_ms,
            open_interest: r.open_interest,
            volume_24h: r.volume_24h,
            stats_ts_ms: r.stats_ts_ms,
        })
        .collect())
}

pub async fn funding_matrix_token_count(client: &Client) -> Result<u64> {
    let row = client
        .query(
            r#"
            SELECT countDistinct(m.token_id) AS cnt
            FROM markets AS m FINAL
            INNER JOIN exchanges AS e FINAL ON e.id = m.exchange_id
            WHERE m.is_active = 1 AND e.is_active = 1
            "#,
        )
        .fetch_one::<CountRow>()
        .await?;
    Ok(row.cnt)
}

pub async fn upsert_exchange(client: &Client, id: i32, name: &str, is_active: bool) -> Result<()> {
    let mut insert = client
        .insert("exchanges")
        .context("opening ClickHouse exchanges insert")?;
    insert
        .write(&ExchangeRow {
            id,
            name: name.to_string(),
            is_active: bool_to_u8(is_active),
            updated_at_ms: now_ms(),
        })
        .await?;
    insert.end().await?;
    Ok(())
}

pub async fn upsert_tokens(client: &Client, rows: &[(i32, String)]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client
        .insert("tokens")
        .context("opening ClickHouse tokens insert")?;
    for (id, symbol) in rows {
        insert
            .write(&TokenRow {
                id: *id,
                symbol: symbol.clone(),
            })
            .await?;
    }
    insert.end().await?;
    Ok(())
}

pub async fn upsert_markets(client: &Client, rows: &[MarketUpsert]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client
        .insert("markets")
        .context("opening ClickHouse markets insert")?;
    let updated_at_ms = now_ms();
    for r in rows {
        insert
            .write(&MarketRow {
                id: r.id,
                exchange_id: r.exchange_id,
                token_id: r.token_id,
                market_symbol: r.market_symbol.clone(),
                is_active: bool_to_u8(r.is_active),
                updated_at_ms,
            })
            .await?;
    }
    insert.end().await?;
    Ok(())
}

pub async fn upsert_cex_exchange(
    client: &Client,
    id: i32,
    name: &str,
    is_active: bool,
) -> Result<()> {
    let mut insert = client
        .insert("cex_exchanges")
        .context("opening ClickHouse cex_exchanges insert")?;
    insert
        .write(&CexExchangeRow {
            id,
            name: name.to_string(),
            is_active: bool_to_u8(is_active),
            updated_at_ms: now_ms(),
        })
        .await?;
    insert.end().await?;
    Ok(())
}

pub async fn upsert_cex_markets(client: &Client, rows: &[CexMarketUpsert]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client
        .insert("cex_markets")
        .context("opening ClickHouse cex_markets insert")?;
    let updated_at_ms = now_ms();
    for r in rows {
        insert
            .write(&CexMarketRow {
                id: r.id,
                exchange_id: r.exchange_id,
                symbol: r.symbol.clone(),
                market_symbol: r.market_symbol.clone(),
                base_asset: r.base_asset.clone(),
                quote_asset: r.quote_asset.clone(),
                market_type: r.market_type.clone(),
                is_active: bool_to_u8(r.is_active),
                updated_at_ms,
            })
            .await?;
    }
    insert.end().await?;
    Ok(())
}

pub async fn insert_funding_rates(
    client: &Client,
    exchange_id: i32,
    rows: &[(i32, &NormalizedFundingRate)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client
        .insert("funding_rates")
        .context("opening ClickHouse funding_rates insert")?;
    for (market_id, r) in rows {
        insert
            .write(&FundingRateRow {
                exchange_id,
                market_id: *market_id,
                rate: decimal_to_f64(&r.rate)?,
                ts_ms: r.timestamp.timestamp_millis(),
            })
            .await?;
    }
    insert.end().await?;
    Ok(())
}

pub async fn insert_market_stats(
    client: &Client,
    rows: &[(i32, &NormalizedMarketStats)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client
        .insert("market_stats")
        .context("opening ClickHouse market_stats insert")?;
    for (market_id, s) in rows {
        insert
            .write(&MarketStatRow {
                market_id: *market_id,
                open_interest: s.open_interest.as_ref().map(decimal_to_f64).transpose()?,
                volume_24h: s.volume_24h.as_ref().map(decimal_to_f64).transpose()?,
                ts_ms: s.timestamp.timestamp_millis(),
            })
            .await?;
    }
    insert.end().await?;
    Ok(())
}

pub async fn insert_cex_klines(
    client: &Client,
    klines: &[(i32, NormalizedKline)],
    interval: &str,
) -> Result<()> {
    if klines.is_empty() {
        return Ok(());
    }

    match interval {
        "1d" => {
            let mut insert = client
                .insert("klines_daily")
                .context("opening ClickHouse klines_daily insert")?;
            for (market_id, k) in klines {
                let date = k.open_time.date_naive();
                let date_key =
                    date.year() * 10_000 + (date.month() as i32) * 100 + date.day() as i32;
                insert
                    .write(&KlineDailyRow {
                        market_id: *market_id,
                        date_key,
                        open_px: decimal_to_f64(&k.open)?,
                        high_px: decimal_to_f64(&k.high)?,
                        low_px: decimal_to_f64(&k.low)?,
                        close_px: decimal_to_f64(&k.close)?,
                        volume: decimal_to_f64(&k.volume)?,
                    })
                    .await?;
            }
            insert.end().await?;
        }
        "1h" => {
            let mut insert = client
                .insert("klines_hourly")
                .context("opening ClickHouse klines_hourly insert")?;
            for (market_id, k) in klines {
                insert
                    .write(&KlineHourlyRow {
                        market_id: *market_id,
                        time_ms: k.open_time.timestamp_millis(),
                        open_px: decimal_to_f64(&k.open)?,
                        high_px: decimal_to_f64(&k.high)?,
                        low_px: decimal_to_f64(&k.low)?,
                        close_px: decimal_to_f64(&k.close)?,
                        volume: decimal_to_f64(&k.volume)?,
                    })
                    .await?;
            }
            insert.end().await?;
        }
        _ => bail!("Unsupported kline interval for ClickHouse insert: {interval}"),
    }

    Ok(())
}

pub async fn insert_cex_trades(client: &Client, trades: &[(i32, NormalizedTrade)]) -> Result<()> {
    if trades.is_empty() {
        return Ok(());
    }

    let mut insert = client
        .insert("trades")
        .context("opening ClickHouse trades insert")?;
    for (market_id, t) in trades {
        insert
            .write(&TradeRow {
                market_id: *market_id,
                trade_id: t.trade_id.clone(),
                trade_time_ms: t.trade_time.timestamp_millis(),
                side: t.side.to_lowercase(),
                price: decimal_to_f64(&t.price)?,
                qty: decimal_to_f64(&t.qty)?,
                quote_qty: decimal_to_f64(&t.quote_qty)?,
            })
            .await?;
    }
    insert.end().await?;
    Ok(())
}

fn validate_identifier(name: &str) -> Result<()> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!("Invalid ClickHouse identifier `{name}`. Use only letters, numbers, and underscore.");
    }
    Ok(())
}

fn decimal_to_f64(v: &Decimal) -> Result<f64> {
    v.to_f64()
        .ok_or_else(|| anyhow!("cannot convert decimal `{v}` to f64"))
}

fn bool_to_u8(v: bool) -> u8 {
    if v { 1 } else { 0 }
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn stable_id(parts: &[&str]) -> i32 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for b in part.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0001_0000_01b3);
        }
        h ^= 0xff;
        h = h.wrapping_mul(0x0000_0001_0000_01b3);
    }

    let v = (h % (i32::MAX as u64 - 1)) + 1;
    v as i32
}

fn date_key_to_midnight_ms(date_key: i32) -> Option<i64> {
    let year = date_key / 10_000;
    let month = ((date_key / 100) % 100) as u32;
    let day = (date_key % 100) as u32;

    let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    let dt = date.and_hms_opt(0, 0, 0)?;
    Some(dt.and_utc().timestamp_millis())
}
