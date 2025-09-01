use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::info;

use crate::cex::common::CexMarketType;
use crate::exchanges::shared::time::TimeSpec;
use crate::sync::{
    funding::sync_funding,
    markets::sync_markets,
    stats::sync_stats,
    cex::{run_cex_sync, CexSyncTask},
};
use crate::utils::cli::CliSource;

// ---------------------------
// Workflow 1: Funding (DEX)
// ---------------------------
pub async fn run_funding_pipeline(pool: &PgPool, exchange: Option<String>, spec: TimeSpec) -> Result<()> {
    info!("--- STARTING FUNDING WORKFLOW ---");
    let target = exchange.clone().unwrap_or_else(|| "all DEXs".to_string());

    info!("[1/3] Syncing DEX Markets for {}...", &target);
    sync_markets(pool, exchange.clone()).await.context("DEX markets sync failed")?;

    info!("[2/3] Syncing Funding Rates for {}...", &target);
    sync_funding(pool, exchange.clone(), spec).await.context("Funding sync failed")?;

    info!("[3/3] Syncing Market Stats for {}...", &target);
    sync_stats(pool, exchange).await.context("Stats sync failed")?;

    info!("--- FUNDING WORKFLOW COMPLETED ---");
    Ok(())
}

// ---------------------------
// Workflow 2: Trend (CEX)
// ---------------------------
pub async fn run_trend_pipeline(
    pool: &PgPool,
    exchange: String,
    market_type: CexMarketType,
    spec: TimeSpec,
    source: CliSource,
    quote: Option<String>,
    all_quotes: bool,
) -> Result<()> {
    info!("--- STARTING TREND WORKFLOW for {} ({:?}) ---", &exchange, market_type);

    // Optional quote filter (None = ALL quotes)
    let selected_quote: Option<String> = if all_quotes {
        None
    } else {
        Some(quote.unwrap_or_else(|| std::env::var("CEX_QUOTE").unwrap_or_else(|_| "USDT".to_string())))
    };

    info!("[1/2] Syncing CEX Markets...");
    run_cex_sync(
        pool,
        CexSyncTask::RefreshMarkets {
            exchange: exchange.clone(),
            selected_quote,
            market_type,
        },
    )
    .await
    .context("CEX markets sync failed")?;

    // Trend uses daily klines; honor source==Klines|Both
    if matches!(source, CliSource::Klines | CliSource::Both) {
        info!("[2/2] Syncing Daily Klines for {}...", &exchange);
        run_cex_sync(
            pool,
            CexSyncTask::SyncKlines {
                exchange: exchange.clone(),
                market_type,
                interval: "1d".to_string(),
                time_spec: spec,
            },
        )
        .await?;
    } else {
        info!("[2/2] Skipping klines (source=trades)");
    }

    info!("--- TREND WORKFLOW COMPLETED ---");
    Ok(())
}

// ---------------------------
// Workflow 3: Z-Score (CEX)
// ---------------------------
pub async fn run_zscore_pipeline(
    pool: &PgPool,
    exchange: String,
    market_type: CexMarketType,
    spec: TimeSpec,
    source: CliSource,
    quote: Option<String>,
    all_quotes: bool,
) -> Result<()> {
    info!("--- STARTING Z-SCORE WORKFLOW for {} ({:?}) ---", &exchange, market_type);

    let selected_quote: Option<String> = if all_quotes {
        None
    } else {
        Some(quote.unwrap_or_else(|| std::env::var("CEX_QUOTE").unwrap_or_else(|_| "USDT".to_string())))
    };

    info!("[1/3] Syncing CEX Markets...");
    run_cex_sync(
        pool,
        CexSyncTask::RefreshMarkets {
            exchange: exchange.clone(),
            selected_quote,
            market_type,
        },
    )
    .await
    .context("CEX markets sync failed")?;

    if matches!(source, CliSource::Klines | CliSource::Both) {
        info!("[2/3] Syncing Hourly Klines for {}...", &exchange);
        run_cex_sync(
            pool,
            CexSyncTask::SyncKlines {
                exchange: exchange.clone(),
                market_type,
                interval: "1h".to_string(),
                time_spec: spec.clone(),
            },
        )
        .await?;
    } else {
        info!("[2/3] Skipping klines (source=trades)");
    }

    if matches!(source, CliSource::Trades | CliSource::Both) {
        info!("[3/3] Syncing Trades for {}...", &exchange);
        run_cex_sync(
            pool,
            CexSyncTask::SyncTrades {
                exchange,
                market_type,
                time_spec: spec,
            },
        )
        .await?;
    } else {
        info!("[3/3] Skipping trades (source=klines)");
    }

    info!("--- Z-SCORE WORKFLOW COMPLETED ---");
    Ok(())
}
