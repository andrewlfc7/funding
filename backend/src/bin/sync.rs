use anyhow::{Context, Result};
use clap::Parser;

use backend::{
    cex::common::CexMarketType,
    db::migrations,
    sync::{
        cex::{CexSyncTask, run_cex_sync},
        pipelines::{run_funding_pipeline, run_trend_pipeline, run_zscore_pipeline},
    },
    utils::cli::{Cli, CliMarketType, CliSource, Command, Workflow},
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let cli = Cli::parse();
    let client = migrations::create_pool().await?;

    match cli.command {
        Command::Sync { workflow } => match workflow {
            Workflow::Funding {
                exchange,
                time_spec,
            } => {
                let spec = time_spec.to_time_spec()?;
                run_funding_pipeline(&client, exchange, spec).await
            }

            Workflow::Trend {
                exchange,
                market_type,
                time_spec,
                source,
                quote,
                all_quotes,
            } => {
                let spec = time_spec.to_time_spec()?;
                let market_type = to_cex_market_type(market_type);
                run_trend_pipeline(
                    &client,
                    exchange,
                    market_type,
                    spec,
                    source,
                    quote,
                    all_quotes,
                )
                .await
            }

            Workflow::Zscore {
                exchange,
                market_type,
                time_spec,
                source,
                quote,
                all_quotes,
            } => {
                let spec = time_spec.to_time_spec()?;
                let market_type = to_cex_market_type(market_type);
                run_zscore_pipeline(
                    &client,
                    exchange,
                    market_type,
                    spec,
                    source,
                    quote,
                    all_quotes,
                )
                .await
            }
        },

        Command::CexAdd {
            name,
            market_type,
            workflow,
            time_spec,
            quote,
            all_quotes,
        } => {
            let spec = time_spec.to_time_spec()?;
            let market_type = to_cex_market_type(market_type);

            let selected_quote = if all_quotes {
                None
            } else {
                Some(quote.clone().unwrap_or_else(|| {
                    std::env::var("CEX_QUOTE").unwrap_or_else(|_| "USDT".to_string())
                }))
            };

            run_cex_sync(
                &client,
                CexSyncTask::RefreshMarkets {
                    exchange: name.clone(),
                    selected_quote,
                    market_type,
                },
            )
            .await
            .context("failed to refresh markets")?;

            match workflow.as_str() {
                "trend" => {
                    run_trend_pipeline(
                        &client,
                        name,
                        market_type,
                        spec,
                        CliSource::Klines,
                        quote,
                        all_quotes,
                    )
                    .await
                }
                "zscore" => {
                    run_zscore_pipeline(
                        &client,
                        name,
                        market_type,
                        spec,
                        CliSource::Both,
                        quote,
                        all_quotes,
                    )
                    .await
                }
                _ => Err(anyhow::anyhow!("Unsupported workflow: {}", workflow)),
            }
        }
    }
    .context("Workflow failed")
}

fn to_cex_market_type(value: CliMarketType) -> CexMarketType {
    match value {
        CliMarketType::Spot => CexMarketType::Spot,
        CliMarketType::Perps => CexMarketType::Perps,
    }
}
