use super::common::ensure_exchange_row;
use crate::data::coin::{SUPPORTED_DEX_EXCHANGES, refresh_markets_for_exchange};
use ::clickhouse::Client;
use anyhow::{Context, Result};
use tracing::{info, warn};

pub async fn sync_markets(client: &Client, exchange_opt: Option<String>) -> Result<()> {
    match exchange_opt {
        Some(ex) => {
            let (id, dbname) = ensure_exchange_row(client, &ex).await?;
            info!("markets: syncing {} (id={})", dbname, id);
            refresh_markets_for_exchange(client, id, &dbname)
                .await
                .context("refresh_markets_for_exchange failed")?;
        }
        None => {
            info!("markets: syncing all configured exchanges");
            for ex in SUPPORTED_DEX_EXCHANGES {
                let (id, dbname) = ensure_exchange_row(client, ex).await?;
                if let Err(e) = refresh_markets_for_exchange(client, id, &dbname).await {
                    warn!(
                        "refresh_markets_for_exchange failed for {} (id={}): {}",
                        dbname, id, e
                    );
                }
            }
        }
    }
    Ok(())
}
