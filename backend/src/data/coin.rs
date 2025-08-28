// src/data/coin.rs
use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::db::insert::upsert_markets;
use crate::exchanges::shared::types::NormalizedMarket;

use crate::exchanges::paradex::api::{client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv};
use crate::exchanges::extended::api::{client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv};
use crate::exchanges::paradex::handler::handler::parse_paradex_markets;
use crate::exchanges::extended::handler::handler::parse_extended_markets;



#[inline]
fn lower(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}


enum ExchangeAdapter {
    Paradex(ParadexClient),
    Extended(ExtendedClient),
}

impl ExchangeAdapter {
    async fn fetch_markets(&self) -> Result<Vec<NormalizedMarket>> {
        match self {
            ExchangeAdapter::Paradex(c) => {
                let raw = c.get_markets().await?;
                parse_paradex_markets(&raw)
            }
            ExchangeAdapter::Extended(c) => {
                let raw = c.get_markets(None).await?;
                parse_extended_markets(&raw)
            }
        }
    }

    fn canonical_name(&self) -> &'static str {
        match self {
            ExchangeAdapter::Paradex(_) => "Paradex",
            ExchangeAdapter::Extended(_) => "Extended",
        }
    }
}

fn make_adapter(name: &str) -> Option<ExchangeAdapter> {
    match lower(name).as_str() {
        "paradex" => Some(ExchangeAdapter::Paradex(ParadexClient::new(ParadexEnv::Mainnet))),
        "extended" => Some(ExchangeAdapter::Extended(ExtendedClient::new(ExtendedEnv::Mainnet))),
        _ => None, // Unknown/unsupported exchange: skip
    }
}


pub async fn refresh_all_markets(pool: &PgPool) -> Result<()> {
    let exchanges = sqlx::query!("SELECT id, name FROM exchanges WHERE is_active = true ORDER BY name")
        .fetch_all(pool)
        .await?;

    for exch in exchanges {
        if let Some(adapter) = make_adapter(&exch.name) {
            let out = adapter.fetch_markets().await?;
            if !out.is_empty() {
                upsert_markets(pool, exch.id, &out).await?;
            }
            info!(
                "upserted {} markets for {} (exchange_id={})",
                out.len(),
                adapter.canonical_name(),
                exch.id
            );
        } else {
            info!("skipping unsupported exchange '{}'(id={})", exch.name, exch.id);
        }
    }

    Ok(())
}


pub async fn refresh_markets_for_exchange(
    pool: &PgPool,
    exchange_id: i32,
    exchange_name: &str,
) -> Result<()> {
    if let Some(adapter) = make_adapter(exchange_name) {
        let out = adapter.fetch_markets().await?;
        if !out.is_empty() {
            upsert_markets(pool, exchange_id, &out).await?;
        }
        info!(
            "upserted {} markets for {} (exchange_id={})",
            out.len(),
            adapter.canonical_name(),
            exchange_id
        );
    } else {
        info!("unsupported exchange '{}'(id={}) — nothing to do", exchange_name, exchange_id);
    }
    Ok(())
}
