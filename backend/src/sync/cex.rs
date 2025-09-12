use anyhow::Result;
use sqlx::PgPool;
use crate::exchanges::shared::time::TimeSpec;
use crate::cex::common::CexMarketType;
use crate::cex::klines::sync_klines_with_spec;
use crate::cex::trades::sync_trades_with_spec;
use crate::cex::markets::refresh_cex_markets;

pub enum CexSyncTask {
    RefreshMarkets {
        exchange: String,
        selected_quote: Option<String>,
        market_type: CexMarketType,
    },
    SyncKlines {
        exchange: String,
        market_type: CexMarketType,
        interval: String,
        time_spec: TimeSpec,
    },
    SyncTrades {
        exchange: String,
        market_type: CexMarketType,
        time_spec: TimeSpec,
    },
}

pub async fn run_cex_sync(pool: &PgPool, task: CexSyncTask) -> Result<()> {
    match task {
        CexSyncTask::RefreshMarkets { exchange, selected_quote, market_type } => {
            let quote_opt = selected_quote.as_deref();
            refresh_cex_markets(pool, &exchange, quote_opt, market_type).await?;
        }
        CexSyncTask::SyncKlines { exchange, market_type, interval, time_spec } => {
            sync_klines_with_spec(pool, &exchange, market_type, &interval, time_spec).await?;
        }
        CexSyncTask::SyncTrades { exchange, market_type, time_spec } => {
            sync_trades_with_spec(pool, &exchange, market_type, time_spec).await?;
        }
    }
    Ok(())
}
