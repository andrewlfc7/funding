use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::info;
use crate::data::coin::{refresh_all_markets, refresh_markets_for_exchange};
use super::common::{ensure_exchange_row};

pub async fn sync_markets(pool: &PgPool, exchange_opt: Option<String>) -> Result<()> {
    match exchange_opt {
        Some(ex) => {
            let (id, dbname) = ensure_exchange_row(pool, &ex).await?;
            info!("markets: syncing {} (id={})", dbname, id);
            refresh_markets_for_exchange(pool, id, &dbname)
                .await
                .context("refresh_markets_for_exchange failed")?;
        }
        None => {
            info!("markets: syncing all active exchanges");
            refresh_all_markets(pool)
                .await
                .context("refresh_all_markets failed")?;
        }
    }
    Ok(())
}