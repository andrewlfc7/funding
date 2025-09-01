// src/data/stats.rs
use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::db::insert::insert_market_stats;
use crate::exchanges::shared::types::NormalizedMarketStats;
use crate::exchanges::hyperliquid::api::{client::HyperliquidClient, endpoints::ApiEnvironment as HyperliquidEnv};
use crate::exchanges::paradex::api::{client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv};
use crate::exchanges::extended::api::{client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv};
use crate::exchanges::paradex::handler::handler::parse_paradex_market_stats;
use crate::exchanges::extended::handler::handler::parse_extended_market_stats;
use crate::exchanges::hyperliquid::handler::handler::parse_hyperliquid_market_stats;

use crate::exchanges::hibachi::api::{client::HibachiClient, endpoints::ApiEnvironment as HibachiEnv};
use crate::exchanges::hibachi::handler::handler::parse_hibachi_market_stats;

use crate::exchanges::bluefin::api::{client::BluefinClient, endpoints::ApiEnvironment as BluefinEnv};
use crate::exchanges::bluefin::handler::handler::parse_bluefin_market_stats; 

use crate::exchanges::drift::api::{client::DriftClient, endpoints::ApiEnvironment as DriftEnv}; 
use crate::exchanges::drift::handler::handler::parse_drift_market_stats; 


#[inline]
fn lower(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

/* ---------------- Exchange adapter ---------------- */

enum StatsAdapter {
    Paradex(ParadexClient),
    Extended(ExtendedClient),
    Hyperliquid(HyperliquidClient), 
    Hibachi(HibachiClient), // Added
    Bluefin(BluefinClient),
    Drift(DriftClient),


}

impl StatsAdapter {
    async fn fetch_one(&self, market_symbol: &str) -> Result<Option<NormalizedMarketStats>> {
        match self {
            StatsAdapter::Paradex(c) => {
                let raw = c.get_markets_summary(market_symbol).await?;
                let stats_vec = parse_paradex_market_stats(&raw)?;
                Ok(stats_vec
                    .into_iter()
                    .find(|s| s.market_symbol == market_symbol))
            }
            StatsAdapter::Extended(c) => {
                let raw = c.get_market_stats(market_symbol).await?;
                let stat = parse_extended_market_stats(&raw, market_symbol)?;
                Ok(Some(stat))
            }
            StatsAdapter::Hyperliquid(c) => {
                let raw = c.get_meta_and_asset_ctxs(None).await?;
                let all_stats = parse_hyperliquid_market_stats(&raw)?;
                Ok(all_stats
                    .into_iter()
                    .find(|s| s.market_symbol == market_symbol))
            }

            StatsAdapter::Hibachi(c) => {
                let raw_oi = c.get_open_interest(market_symbol).await?;
                let raw_stats = c.get_stats(market_symbol).await?;
                let raw_prices = c.get_prices(market_symbol).await?;
                
                let stat = parse_hibachi_market_stats(&raw_oi, &raw_stats, &raw_prices, market_symbol)?;
                Ok(Some(stat))
            }
            StatsAdapter::Bluefin(c) => {
                let raw = c.get_tickers().await?;
                let all_stats = parse_bluefin_market_stats(&raw)?;
                Ok(all_stats
                    .into_iter()
                    .find(|s| s.market_symbol == market_symbol))
            }
            StatsAdapter::Drift(c) => {
                let raw = c.get_contracts().await?;
                let all_stats = parse_drift_market_stats(&raw)?;
                Ok(all_stats
                    .into_iter()
                    .find(|s| s.market_symbol == market_symbol))
            }

        }
    }

    fn canonical_name(&self) -> &'static str {
        match self {
            StatsAdapter::Paradex(_) => "Paradex",
            StatsAdapter::Extended(_) => "Extended",
            StatsAdapter::Hyperliquid(_) => "Hyperliquid",
            StatsAdapter::Hibachi(_) => "Hibachi", 
            StatsAdapter::Bluefin(_) => "Bluefin",
            StatsAdapter::Drift(_) => "Drift",

        }
    }
}

fn make_stats_adapter(name: &str) -> Option<StatsAdapter> {
    match lower(name).as_str() {
        "paradex" => Some(StatsAdapter::Paradex(ParadexClient::new(ParadexEnv::Mainnet))),
        "extended" => Some(StatsAdapter::Extended(ExtendedClient::new(ExtendedEnv::Mainnet))),
        "hyperliquid" => Some(StatsAdapter::Hyperliquid(HyperliquidClient::new(HyperliquidEnv::Mainnet))),
        "hibachi" => Some(StatsAdapter::Hibachi(HibachiClient::new(HibachiEnv::Mainnet))),
        "bluefin" => Some(StatsAdapter::Bluefin(BluefinClient::new(BluefinEnv::Mainnet))),
        "drift" => Some(StatsAdapter::Drift(DriftClient::new(DriftEnv::Mainnet))),

        _ => None,
    }
}

pub async fn collect_daily_market_stats(pool: &PgPool) -> Result<()> {
    let exchanges = sqlx::query!(
        "SELECT id, name FROM exchanges WHERE is_active = true ORDER BY name"
    )
    .fetch_all(pool)
    .await?;

    for exch in exchanges {
        let Some(adapter) = make_stats_adapter(&exch.name) else {
            warn!("stats: skipping unsupported exchange '{}'(id={})", exch.name, exch.id);
            continue;
        };

        let markets = sqlx::query!(
            "SELECT id, market_symbol FROM markets WHERE exchange_id = $1 AND is_active = true ORDER BY market_symbol",
            exch.id
        )
        .fetch_all(pool)
        .await?;

        let mut owned: Vec<(i32, NormalizedMarketStats)> = Vec::with_capacity(markets.len());

        for m in markets {
            if let Some(stat) = adapter.fetch_one(&m.market_symbol).await? {
                owned.push((m.id, stat));
            }
        }

        if !owned.is_empty() {
            let borrowed: Vec<(i32, &NormalizedMarketStats)> =
                owned.iter().map(|(mid, s)| (*mid, s)).collect();
            insert_market_stats(pool, &borrowed).await?;
            info!(
                "stats: inserted {} rows for {} (exchange_id={})",
                borrowed.len(),
                adapter.canonical_name(),
                exch.id
            );
        } else {
            info!(
                "stats: no rows to insert for {} (exchange_id={})",
                adapter.canonical_name(),
                exch.id
            );
        }
    }

    Ok(())
}