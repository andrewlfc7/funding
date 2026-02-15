use ::clickhouse::Client;
use anyhow::Result;
use tracing::info;

use super::common::ensure_exchange_row;
use crate::data::coin::SUPPORTED_DEX_EXCHANGES;
use crate::data::stats::{collect_daily_market_stats, collect_market_stats_for_exchange};

pub async fn sync_stats(client: &Client, exchange_opt: Option<String>) -> Result<()> {
    match exchange_opt {
        Some(ex) => {
            let (id, dbname) = ensure_exchange_row(client, &ex).await?;
            info!("stats: syncing {} (id={})", dbname, id);
            collect_market_stats_for_exchange(client, id, &dbname).await?;
        }
        None => {
            info!("stats: syncing all configured exchanges");
            for ex in SUPPORTED_DEX_EXCHANGES {
                let (_id, _dbname) = ensure_exchange_row(client, ex).await?;
            }
            collect_daily_market_stats(client).await?;
        }
    }
    Ok(())
}
