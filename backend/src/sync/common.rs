use anyhow::{anyhow, Result};
use sqlx::PgPool;

// --- Enums ---
#[derive(PartialEq, Clone, Copy)]
pub enum RunMode {
    Backfill,
    Normal,
}

// --- Helper Functions ---
#[inline]
pub fn lower(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

pub async fn lookup_exchange_id_case_insensitive(
    pool: &PgPool,
    name: &str,
) -> Result<Option<(i32, String)>> {
    let rec = sqlx::query!(
        r#"
        SELECT id, name
        FROM exchanges
        WHERE is_active = true AND lower(name) = lower($1)
        "#,
        name
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec.map(|r| (r.id, r.name)))
}

pub async fn ensure_exchange_row(pool: &PgPool, name: &str) -> Result<(i32, String)> {
    if let Some((id, dbname)) = lookup_exchange_id_case_insensitive(pool, name).await? {
        return Ok((id, dbname));
    }

    let inserted = sqlx::query!(
        r#"
        INSERT INTO exchanges (name, is_active)
        VALUES ($1, true)
        ON CONFLICT (name) DO UPDATE
            SET is_active = EXCLUDED.is_active,
                updated_at = NOW()
        RETURNING id, name
        "#,
        name
    )
    .fetch_one(pool)
    .await?;

    Ok((inserted.id, inserted.name))
}


pub async fn fetch_market_symbols_from_api(exchange_name: &str) -> Result<Vec<String>> {
    match lower(exchange_name).as_str() {
        "paradex" => {
            use crate::exchanges::paradex::api::{client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv};
            use crate::exchanges::paradex::handler::handler::parse_paradex_markets;
            let client = ParadexClient::new(ParadexEnv::Mainnet);
            let raw = client.get_markets().await?;
            let markets = parse_paradex_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }
        "extended" => {
            use crate::exchanges::extended::api::{client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv};
            use crate::exchanges::extended::handler::handler::parse_extended_markets;
            let client = ExtendedClient::new(ExtendedEnv::Mainnet);
            let raw = client.get_markets(None).await?;
            let markets = parse_extended_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }
        "hyperliquid" => {
            use crate::exchanges::hyperliquid::api::{client::HyperliquidClient, endpoints::ApiEnvironment as HyperliquidEnv};
            use crate::exchanges::hyperliquid::handler::handler::parse_hyperliquid_markets;
            let client = HyperliquidClient::new(HyperliquidEnv::Mainnet);
            let raw = client.get_perp_meta(None).await?;
            let markets = parse_hyperliquid_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }

        "hibachi" => {
            use crate::exchanges::hibachi::api::{client::HibachiClient, endpoints::ApiEnvironment as HibachiEnv};
            use crate::exchanges::hibachi::handler::handler::parse_hibachi_markets;
            let client = HibachiClient::new(HibachiEnv::Mainnet);
            let raw = client.get_exchange_info().await?;
            let markets = parse_hibachi_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }
        
        "bluefin" => {
            use crate::exchanges::bluefin::api::{client::BluefinClient, endpoints::ApiEnvironment as BluefinEnv};
            use crate::exchanges::bluefin::handler::handler::parse_bluefin_markets;
            let client = BluefinClient::new(BluefinEnv::Mainnet);
            let raw = client.get_exchange_info().await?;
            let markets = parse_bluefin_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }
        "drift" => {
            use crate::exchanges::drift::api::{client::DriftClient, endpoints::ApiEnvironment as DriftEnv};
            use crate::exchanges::drift::handler::handler::parse_drift_markets;
            let client = DriftClient::new(DriftEnv::Mainnet);
            let raw = client.get_contracts().await?;
            let markets = parse_drift_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }

        other => anyhow::bail!("unsupported exchange '{}'", other),
    }
}



