use std::collections::HashMap;
use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::data::stats::collect_daily_market_stats;
use crate::db::insert::{insert_market_stats, insert_market_stats_by_symbol};
use crate::exchanges::shared::types::NormalizedMarketStats;
use super::common::{lookup_exchange_id_case_insensitive, fetch_market_symbols_from_api, RunMode, lower};



pub async fn sync_stats(pool: &PgPool, exchange_opt: Option<String>) -> Result<()> {
    match exchange_opt {
        Some(ex) => {
            let (id, dbname) = lookup_exchange_id_case_insensitive(pool, &ex)
                .await?
                .ok_or_else(|| anyhow!("exchange not found or inactive: {}", ex))?;
            info!("stats: syncing {} (id={})", dbname, id);
            collect_stats_for_single_exchange(pool, id, &dbname, RunMode::Normal).await?;
        }
        None => {
            info!("stats: syncing all active exchanges");
            collect_daily_market_stats(pool)
                .await
                .context("collect_daily_market_stats failed")?;
        }
    }
    Ok(())
}

pub async fn collect_stats_for_single_exchange(
    pool: &PgPool,
    exchange_id: i32,
    exchange_name: &str,
    mode: RunMode,
) -> Result<()> {
    match lower(exchange_name).as_str() {
        "paradex" => {
            use crate::exchanges::paradex::api::{client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv};
            use crate::exchanges::paradex::handler::handler::parse_paradex_market_stats;
            let client = ParadexClient::new(ParadexEnv::Mainnet);

            match mode {
                RunMode::Backfill => {
                    let symbols = fetch_market_symbols_from_api(exchange_name).await?;
                    let mut owned: Vec<(String, NormalizedMarketStats)> = Vec::with_capacity(symbols.len());
                    for sym in symbols {
                        let raw = client.get_markets_summary(&sym).await?;
                        let stats_vec = parse_paradex_market_stats(&raw)?;
                        if let Some(stat) = stats_vec.into_iter().find(|s| s.market_symbol == sym) {
                            owned.push((sym, stat));
                        }
                    }
                    if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(s, r)| (s.clone(), r)).collect::<Vec<_>>();
                        insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                        info!("stats/backfill: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
                RunMode::Normal => {
                    let markets = sqlx::query!("SELECT id, market_symbol FROM markets WHERE exchange_id = $1 AND is_active = true", exchange_id).fetch_all(pool).await?;
                    let mut owned: Vec<(i32, NormalizedMarketStats)> = Vec::with_capacity(markets.len());
                    for m in markets {
                        let raw = client.get_markets_summary(&m.market_symbol).await?;
                        let stats_vec = parse_paradex_market_stats(&raw)?;
                        if let Some(stat) = stats_vec.into_iter().find(|s| s.market_symbol == m.market_symbol) {
                            owned.push((m.id, stat));
                        }
                    }
                    if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(mid, s)| (*mid, s)).collect::<Vec<_>>();
                        insert_market_stats(pool, &borrowed).await?;
                        info!("stats: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
            }
        }
        "extended" => {
            use crate::exchanges::extended::api::{client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv};
            use crate::exchanges::extended::handler::handler::parse_extended_market_stats;
            let client = ExtendedClient::new(ExtendedEnv::Mainnet);

            match mode {
                RunMode::Backfill => {
                    let symbols = fetch_market_symbols_from_api(exchange_name).await?;
                    let mut owned: Vec<(String, NormalizedMarketStats)> = Vec::with_capacity(symbols.len());
                    for sym in symbols {
                        let raw = client.get_market_stats(&sym).await?;
                        let stat = parse_extended_market_stats(&raw, &sym)?;
                        owned.push((sym, stat));
                    }
                     if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(s, r)| (s.clone(), r)).collect::<Vec<_>>();
                        insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                        info!("stats/backfill: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
                RunMode::Normal => {
                    let markets = sqlx::query!("SELECT id, market_symbol FROM markets WHERE exchange_id = $1 AND is_active = true", exchange_id).fetch_all(pool).await?;
                    let mut owned: Vec<(i32, NormalizedMarketStats)> = Vec::with_capacity(markets.len());
                    for m in markets {
                        let raw = client.get_market_stats(&m.market_symbol).await?;
                        let stat = parse_extended_market_stats(&raw, &m.market_symbol)?;
                        owned.push((m.id, stat));
                    }
                    if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(mid, s)| (*mid, s)).collect::<Vec<_>>();
                        insert_market_stats(pool, &borrowed).await?;
                        info!("stats: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
            }
        }
        "hyperliquid" => {
             use crate::exchanges::hyperliquid::api::{client::HyperliquidClient, endpoints::ApiEnvironment as HyperliquidEnv};
             use crate::exchanges::hyperliquid::handler::handler::parse_hyperliquid_market_stats;
             let client = HyperliquidClient::new(HyperliquidEnv::Mainnet);
             let raw_stats = client.get_meta_and_asset_ctxs(None).await?;
             let all_stats = parse_hyperliquid_market_stats(&raw_stats)?;
             let stats_map: HashMap<String, NormalizedMarketStats> = all_stats
                 .into_iter()
                 .map(|s| (s.market_symbol.clone(), s))
                 .collect();

             match mode {
                 RunMode::Backfill => {
                     let symbols = fetch_market_symbols_from_api(exchange_name).await?;
                     let owned: Vec<(String, NormalizedMarketStats)> = symbols
                         .into_iter()
                         .filter_map(|sym| stats_map.get(&sym).map(|stat| (sym, stat.clone())))
                         .collect();
                    
                     if !owned.is_empty() {
                         let borrowed = owned.iter().map(|(s, r)| (s.clone(), r)).collect::<Vec<_>>();
                         insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                         info!("stats/backfill: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                     }
                 }
                 RunMode::Normal => {
                     let markets = sqlx::query!("SELECT id, market_symbol FROM markets WHERE exchange_id = $1 AND is_active = true", exchange_id).fetch_all(pool).await?;
                     let owned: Vec<(i32, NormalizedMarketStats)> = markets
                         .into_iter()
                         .filter_map(|m| stats_map.get(&m.market_symbol).map(|stat| (m.id, stat.clone())))
                         .collect();
                    
                     if !owned.is_empty() {
                         let borrowed = owned.iter().map(|(mid, s)| (*mid, s)).collect::<Vec<_>>();
                         insert_market_stats(pool, &borrowed).await?;
                         info!("stats: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                     }
                 }
             }
        }

        
        "hibachi" => {
            use crate::exchanges::hibachi::api::{client::HibachiClient, endpoints::ApiEnvironment as HibachiEnv};
            use crate::exchanges::hibachi::handler::handler::parse_hibachi_market_stats;
            
            let client = HibachiClient::new(HibachiEnv::Mainnet);

            let get_market_symbols = |mode: RunMode| async move {
                match mode {
                    RunMode::Backfill => fetch_market_symbols_from_api(exchange_name).await,
                    RunMode::Normal => {
                        let markets = sqlx::query!("SELECT market_symbol FROM markets WHERE exchange_id = $1 AND is_active = true", exchange_id)
                            .fetch_all(pool).await?;
                        Ok(markets.into_iter().map(|r| r.market_symbol).collect())
                    }
                }
            };
            
            let symbols = get_market_symbols(mode).await?;
            let mut owned_stats = Vec::new();

            for symbol in symbols {
                let res = tokio::try_join!(
                    client.get_open_interest(&symbol),
                    client.get_stats(&symbol),
                    client.get_prices(&symbol)
                );
                match res {
                    Ok((raw_oi, raw_stats, raw_prices)) => {
                        let stat = parse_hibachi_market_stats(&raw_oi, &raw_stats, &raw_prices, &symbol)?;
                        owned_stats.push((symbol, stat));
                    }
                    Err(e) => warn!("Failed to fetch all stats for Hibachi market {}: {}", symbol, e),
                }
            }

            if owned_stats.is_empty() {
                if mode == RunMode::Normal { info!("stats: no rows to insert for {} (id={})", exchange_name, exchange_id) } 
                else { info!("stats/backfill: no rows to insert for {} (id={})", exchange_name, exchange_id) };
                return Ok(());
            }

            match mode {
                RunMode::Backfill => {
                    let borrowed = owned_stats.iter().map(|(s, r)| (s.clone(), r)).collect::<Vec<_>>();
                    insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                    info!("stats/backfill: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                }
                RunMode::Normal => {
                    let market_map: HashMap<String, i32> = sqlx::query!("SELECT id, market_symbol FROM markets WHERE exchange_id = $1", exchange_id)
                        .fetch_all(pool).await?.into_iter().map(|r| (r.market_symbol, r.id)).collect();
                    let owned_with_ids: Vec<(i32, NormalizedMarketStats)> = owned_stats.into_iter()
                        .filter_map(|(symbol, stats)| market_map.get(&symbol).map(|id| (*id, stats)))
                        .collect();
                    let borrowed = owned_with_ids.iter().map(|(mid, s)| (*mid, s)).collect::<Vec<_>>();
                    insert_market_stats(pool, &borrowed).await?;
                    info!("stats: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                }
            }
        }

        "bluefin" => {
            use crate::exchanges::bluefin::api::{client::BluefinClient, endpoints::ApiEnvironment as BluefinEnv};
            use crate::exchanges::bluefin::handler::handler::parse_bluefin_market_stats;
            let client = BluefinClient::new(BluefinEnv::Mainnet);

            // Efficiently fetch stats for ALL markets at once
            let raw_stats = client.get_tickers().await?;
            let all_stats = parse_bluefin_market_stats(&raw_stats)?;
            let stats_map: HashMap<String, NormalizedMarketStats> = all_stats
                .into_iter()
                .map(|s| (s.market_symbol.clone(), s))
                .collect();

            match mode {
                RunMode::Backfill => {
                    let symbols = fetch_market_symbols_from_api(exchange_name).await?;
                    let owned: Vec<(String, NormalizedMarketStats)> = symbols
                        .into_iter()
                        .filter_map(|sym| stats_map.get(&sym).map(|stat| (sym, stat.clone())))
                        .collect();
                    if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(s, r)| (s.clone(), r)).collect::<Vec<_>>();
                        insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                        info!("stats/backfill: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
                RunMode::Normal => {
                    let markets = sqlx::query!("SELECT id, market_symbol FROM markets WHERE exchange_id = $1 AND is_active = true", exchange_id).fetch_all(pool).await?;
                    let owned: Vec<(i32, NormalizedMarketStats)> = markets
                        .into_iter()
                        .filter_map(|m| stats_map.get(&m.market_symbol).map(|stat| (m.id, stat.clone())))
                        .collect();
                    if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(mid, s)| (*mid, s)).collect::<Vec<_>>();
                        insert_market_stats(pool, &borrowed).await?;
                        info!("stats: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
            }
        }

        "drift" => {
            use crate::exchanges::drift::api::{client::DriftClient, endpoints::ApiEnvironment as DriftEnv};
            use crate::exchanges::drift::handler::handler::parse_drift_market_stats;
            let client = DriftClient::new(DriftEnv::Mainnet);

            let raw_stats = client.get_contracts().await?;
            let all_stats = parse_drift_market_stats(&raw_stats)?;
            let stats_map: HashMap<String, NormalizedMarketStats> = all_stats
                .into_iter()
                .map(|s| (s.market_symbol.clone(), s))
                .collect();

            match mode {
                RunMode::Backfill => {
                    let symbols = fetch_market_symbols_from_api(exchange_name).await?;
                    let owned: Vec<(String, NormalizedMarketStats)> = symbols
                        .into_iter()
                        .filter_map(|sym| stats_map.get(&sym).map(|stat| (sym, stat.clone())))
                        .collect();
                    if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(s, r)| (s.clone(), r)).collect::<Vec<_>>();
                        insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                        info!("stats/backfill: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
                RunMode::Normal => {
                    let markets = sqlx::query!("SELECT id, market_symbol FROM markets WHERE exchange_id = $1 AND is_active = true", exchange_id).fetch_all(pool).await?;
                    let owned: Vec<(i32, NormalizedMarketStats)> = markets
                        .into_iter()
                        .filter_map(|m| stats_map.get(&m.market_symbol).map(|stat| (m.id, stat.clone())))
                        .collect();
                    if !owned.is_empty() {
                        let borrowed = owned.iter().map(|(mid, s)| (*mid, s)).collect::<Vec<_>>();
                        insert_market_stats(pool, &borrowed).await?;
                        info!("stats: inserted {} rows for {} (id={})", borrowed.len(), exchange_name, exchange_id);
                    }
                }
            }
        }
        _ => warn!("stats: unsupported exchange '{}'", exchange_name),
    }
    
    if mode == RunMode::Normal {
        info!("stats: finished sync for {} (id={})", exchange_name, exchange_id);
    } else {
        info!("stats/backfill: finished sync for {} (id={})", exchange_name, exchange_id);
    }
    
    Ok(())
}