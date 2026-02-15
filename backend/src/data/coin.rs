use ::clickhouse::Client;
use anyhow::Result;
use tracing::info;

use crate::db::insert::{upsert_exchange, upsert_markets};
use crate::exchanges::shared::types::NormalizedMarket;

use crate::exchanges::extended::api::{
    client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv,
};
use crate::exchanges::extended::handler::handler::parse_extended_markets;
use crate::exchanges::hyperliquid::api::{
    client::HyperliquidClient, endpoints::ApiEnvironment as HyperliquidEnv,
};
use crate::exchanges::hyperliquid::handler::handler::parse_hyperliquid_markets;
use crate::exchanges::paradex::api::{
    client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv,
};
use crate::exchanges::paradex::handler::handler::parse_paradex_markets;

use crate::exchanges::hibachi::api::{
    client::HibachiClient, endpoints::ApiEnvironment as HibachiEnv,
};
use crate::exchanges::hibachi::handler::handler::parse_hibachi_markets;

use crate::exchanges::bluefin::api::{
    client::BluefinClient, endpoints::ApiEnvironment as BluefinEnv,
};
use crate::exchanges::bluefin::handler::handler::parse_bluefin_markets;

use crate::exchanges::drift::api::{client::DriftClient, endpoints::ApiEnvironment as DriftEnv};
use crate::exchanges::drift::handler::handler::parse_drift_markets;

pub const SUPPORTED_DEX_EXCHANGES: &[&str] = &[
    "Paradex",
    "Extended",
    "Hyperliquid",
    "Hibachi",
    "Bluefin",
    "Drift",
];

#[inline]
fn lower(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

enum ExchangeAdapter {
    Paradex(ParadexClient),
    Extended(ExtendedClient),
    Hyperliquid(HyperliquidClient),
    Hibachi(HibachiClient),
    Bluefin(BluefinClient),
    Drift(DriftClient),
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
            ExchangeAdapter::Hyperliquid(c) => {
                let raw = c.get_perp_meta(None).await?;
                parse_hyperliquid_markets(&raw)
            }
            ExchangeAdapter::Hibachi(c) => {
                let raw = c.get_exchange_info().await?;
                parse_hibachi_markets(&raw)
            }
            ExchangeAdapter::Bluefin(c) => {
                let raw = c.get_exchange_info().await?;
                parse_bluefin_markets(&raw)
            }
            ExchangeAdapter::Drift(c) => {
                let raw = c.get_contracts().await?;
                parse_drift_markets(&raw)
            }
        }
    }

    fn canonical_name(&self) -> &'static str {
        match self {
            ExchangeAdapter::Paradex(_) => "Paradex",
            ExchangeAdapter::Extended(_) => "Extended",
            ExchangeAdapter::Hyperliquid(_) => "Hyperliquid",
            ExchangeAdapter::Hibachi(_) => "Hibachi",
            ExchangeAdapter::Bluefin(_) => "Bluefin",
            ExchangeAdapter::Drift(_) => "Drift",
        }
    }
}

fn make_adapter(name: &str) -> Option<ExchangeAdapter> {
    match lower(name).as_str() {
        "paradex" => Some(ExchangeAdapter::Paradex(ParadexClient::new(
            ParadexEnv::Mainnet,
        ))),
        "extended" => Some(ExchangeAdapter::Extended(ExtendedClient::new(
            ExtendedEnv::Mainnet,
        ))),
        "hyperliquid" => Some(ExchangeAdapter::Hyperliquid(HyperliquidClient::new(
            HyperliquidEnv::Mainnet,
        ))),
        "hibachi" => Some(ExchangeAdapter::Hibachi(HibachiClient::new(
            HibachiEnv::Mainnet,
        ))),
        "bluefin" => Some(ExchangeAdapter::Bluefin(BluefinClient::new(
            BluefinEnv::Mainnet,
        ))),
        "drift" => Some(ExchangeAdapter::Drift(DriftClient::new(DriftEnv::Mainnet))),
        _ => None,
    }
}

pub async fn fetch_markets_for_exchange(exchange_name: &str) -> Result<Vec<NormalizedMarket>> {
    let adapter = make_adapter(exchange_name)
        .ok_or_else(|| anyhow::anyhow!("unsupported exchange: {}", exchange_name))?;
    adapter.fetch_markets().await
}

pub async fn refresh_all_markets(client: &Client) -> Result<()> {
    for name in SUPPORTED_DEX_EXCHANGES {
        let Some(adapter) = make_adapter(name) else {
            continue;
        };
        let exchange_name = adapter.canonical_name();
        let exchange_id = upsert_exchange(client, exchange_name).await?;
        let out = adapter.fetch_markets().await?;
        if !out.is_empty() {
            upsert_markets(client, exchange_id, &out).await?;
        }
        info!(
            "upserted {} markets for {} (exchange_id={})",
            out.len(),
            exchange_name,
            exchange_id
        );
    }

    Ok(())
}

pub async fn refresh_markets_for_exchange(
    client: &Client,
    exchange_id: i32,
    exchange_name: &str,
) -> Result<()> {
    if let Some(adapter) = make_adapter(exchange_name) {
        let out = adapter.fetch_markets().await?;
        if !out.is_empty() {
            upsert_markets(client, exchange_id, &out).await?;
        }
        info!(
            "upserted {} markets for {} (exchange_id={})",
            out.len(),
            adapter.canonical_name(),
            exchange_id
        );
    } else {
        info!(
            "unsupported exchange '{}'(id={}) - nothing to do",
            exchange_name, exchange_id
        );
    }
    Ok(())
}
