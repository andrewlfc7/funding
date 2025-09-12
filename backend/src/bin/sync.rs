use anyhow::{Context, Result};
use clap::Parser;

use backend::{
    utils::cli::{Cli, Command, Workflow, CliMarketType, CliSource},
    db::migrations,
    exchanges::shared::time::TimeSpec,
    cex::common::CexMarketType,
    sync::{
        pipelines::{run_funding_pipeline, run_trend_pipeline, run_zscore_pipeline},
        cex::{run_cex_sync, CexSyncTask},
    },
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt().with_target(false).compact().init();

    let cli = Cli::parse();
    let pool = migrations::create_pool().await;

    match cli.command {
        Command::Sync { workflow } => {
            match workflow {
                // 1. DEX Funding
                Workflow::Funding { exchange, time_spec } => {
                    let spec: TimeSpec = time_spec.to_time_spec()?;
                    run_funding_pipeline(&pool, exchange, spec).await
                }

                Workflow::Trend { exchange, market_type, time_spec, source, quote, all_quotes } => {
                    let spec: TimeSpec = time_spec.to_time_spec()?;
                    let market_type: CexMarketType = market_type.into();
                    run_trend_pipeline(&pool, exchange, market_type, spec, source, quote, all_quotes).await
                }

                Workflow::Zscore { exchange, market_type, time_spec, source, quote, all_quotes } => {
                    let spec: TimeSpec = time_spec.to_time_spec()?;
                    let market_type: CexMarketType = market_type.into();
                    run_zscore_pipeline(&pool, exchange, market_type, spec, source, quote, all_quotes).await
                }
            }
        }

        Command::CexAdd { name, market_type, workflow, time_spec, quote, all_quotes } => {
            let spec: TimeSpec = time_spec.to_time_spec()?;
            let market_type: CexMarketType = market_type.into();

            let selected_quote = if all_quotes {
                None
            } else {
                Some(quote.clone().unwrap_or_else(|| std::env::var("CEX_QUOTE").unwrap_or_else(|_| "USDT".to_string())))
            };

            run_cex_sync(
                &pool,
                CexSyncTask::RefreshMarkets {
                    exchange: name.clone(),
                    selected_quote,
                    market_type,
                },
            )
            .await
            .context("failed to refresh markets")?;

            match workflow.as_str() {
                "trend" => run_trend_pipeline(&pool, name, market_type, spec, CliSource::Klines, quote, all_quotes).await,
                "zscore" => run_zscore_pipeline(&pool, name, market_type, spec, CliSource::Both, quote, all_quotes).await,
                _ => Err(anyhow::anyhow!("Unsupported workflow: {}", workflow)),
            }
        }
    }
    .context("Workflow failed")
}
