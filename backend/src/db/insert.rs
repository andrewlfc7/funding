use ::clickhouse::Client;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};

use crate::db::clickhouse::{self, CexMarketUpsert, MarketUpsert};
use crate::exchanges::shared::types::{
    CexMarket, NormalizedFundingRate, NormalizedKline, NormalizedMarket, NormalizedMarketStats,
    NormalizedTrade,
};

pub async fn upsert_exchange(client: &Client, name: &str) -> Result<i32> {
    let id = clickhouse::exchange_id(name);
    clickhouse::upsert_exchange(client, id, name, true).await?;
    Ok(id)
}

pub async fn upsert_markets(
    client: &Client,
    exchange_id: i32,
    markets: &[NormalizedMarket],
) -> Result<()> {
    if markets.is_empty() {
        return Ok(());
    }

    let mut tokens = HashSet::with_capacity(markets.len());
    for m in markets {
        tokens.insert(m.symbol.clone());
    }

    let token_rows: Vec<(i32, String)> = tokens
        .into_iter()
        .map(|sym| (clickhouse::token_id(&sym), sym))
        .collect();
    clickhouse::upsert_tokens(client, &token_rows).await?;

    let token_map: HashMap<String, i32> = token_rows
        .iter()
        .map(|(id, sym)| (sym.clone(), *id))
        .collect();

    let market_rows: Vec<MarketUpsert> = markets
        .iter()
        .map(|m| MarketUpsert {
            id: clickhouse::market_id(exchange_id, &m.market_symbol),
            exchange_id,
            token_id: *token_map
                .get(&m.symbol)
                .expect("token must exist in token map"),
            market_symbol: m.market_symbol.clone(),
            is_active: m.is_active,
        })
        .collect();

    clickhouse::upsert_markets(client, &market_rows).await?;
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

    let chunk_size = insert_chunk_size();
    let client = client.clone();
    let rows_owned: Vec<(i32, NormalizedFundingRate)> = rows
        .iter()
        .map(|(market_id, row)| (*market_id, (*row).clone()))
        .collect();

    let handle = tokio::spawn(async move {
        for chunk in rows_owned.chunks(chunk_size) {
            let borrowed: Vec<(i32, &NormalizedFundingRate)> =
                chunk.iter().map(|(mid, r)| (*mid, r)).collect();
            clickhouse::insert_funding_rates(&client, exchange_id, &borrowed).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    handle
        .await
        .context("funding insert worker join failed")??;
    Ok(())
}

pub async fn insert_market_stats(
    client: &Client,
    rows: &[(i32, &NormalizedMarketStats)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let chunk_size = insert_chunk_size();
    let client = client.clone();
    let rows_owned: Vec<(i32, NormalizedMarketStats)> = rows
        .iter()
        .map(|(market_id, row)| (*market_id, (*row).clone()))
        .collect();

    let handle = tokio::spawn(async move {
        for chunk in rows_owned.chunks(chunk_size) {
            let borrowed: Vec<(i32, &NormalizedMarketStats)> =
                chunk.iter().map(|(mid, r)| (*mid, r)).collect();
            clickhouse::insert_market_stats(&client, &borrowed).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    handle
        .await
        .context("market stats insert worker join failed")??;
    Ok(())
}

pub async fn insert_market_stats_by_symbol(
    client: &Client,
    exchange_id: i32,
    rows: &[(String, &NormalizedMarketStats)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let resolved: Vec<(i32, &NormalizedMarketStats)> = rows
        .iter()
        .map(|(sym, stat)| (clickhouse::market_id(exchange_id, sym), *stat))
        .collect();

    insert_market_stats(client, &resolved).await
}

pub async fn insert_cex_klines(
    client: &Client,
    klines: Vec<(i32, NormalizedKline)>,
    interval: &str,
) -> Result<()> {
    if klines.is_empty() {
        return Ok(());
    }

    let chunk_size = insert_chunk_size();
    let client = client.clone();
    let interval = interval.to_string();

    let handle = tokio::spawn(async move {
        for chunk in klines.chunks(chunk_size) {
            clickhouse::insert_cex_klines(&client, chunk, &interval).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    handle
        .await
        .context("cex klines insert worker join failed")??;
    Ok(())
}

pub async fn insert_cex_trades(client: &Client, trades: Vec<(i32, NormalizedTrade)>) -> Result<()> {
    if trades.is_empty() {
        return Ok(());
    }

    let chunk_size = insert_chunk_size();
    let client = client.clone();

    let handle = tokio::spawn(async move {
        for chunk in trades.chunks(chunk_size) {
            clickhouse::insert_cex_trades(&client, chunk).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    handle
        .await
        .context("cex trades insert worker join failed")??;
    Ok(())
}

pub async fn upsert_cex_exchange(client: &Client, name: &str) -> Result<i32> {
    let id = clickhouse::cex_exchange_id(name);
    clickhouse::upsert_cex_exchange(client, id, name, true).await?;
    Ok(id)
}

pub async fn upsert_cex_markets(
    client: &Client,
    exchange_id: i32,
    markets: &[CexMarket],
) -> Result<()> {
    if markets.is_empty() {
        return Ok(());
    }

    let payload: Vec<CexMarketUpsert> = markets
        .iter()
        .map(|m| CexMarketUpsert {
            id: clickhouse::cex_market_id(exchange_id, &m.market_symbol, &m.market_type),
            exchange_id,
            symbol: m.symbol.clone(),
            market_symbol: m.market_symbol.clone(),
            base_asset: m.base_currency.clone(),
            quote_asset: m.quote_currency.clone(),
            market_type: m.market_type.clone(),
            is_active: m.is_active,
        })
        .collect();

    clickhouse::upsert_cex_markets(client, &payload).await
}

fn insert_chunk_size() -> usize {
    std::env::var("SYNC_DB_CHUNK")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(60_000)
}
