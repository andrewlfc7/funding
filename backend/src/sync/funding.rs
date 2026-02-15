use ::clickhouse::Client;
use anyhow::{Context, Result};
use tracing::{error, info};

use super::common::ensure_exchange_row;
use crate::data::coin::SUPPORTED_DEX_EXCHANGES;
use crate::data::funding::collect_funding_for_exchange_with_spec;
use crate::exchanges::shared::time::TimeSpec;

pub async fn sync_funding(
    client: &Client,
    exchange_opt: Option<String>,
    spec: TimeSpec,
) -> Result<()> {
    match exchange_opt {
        Some(ex) => {
            let (id, dbname) = ensure_exchange_row(client, &ex).await?;
            info!("funding: {} window={:?}", dbname, spec);
            collect_funding_for_exchange_with_spec(client, id, &dbname, spec)
                .await
                .with_context(|| format!("funding sync failed for {}", dbname))?;
        }
        None => {
            info!("funding: all configured exchanges, window={:?}", spec);
            for ex in SUPPORTED_DEX_EXCHANGES {
                let (id, dbname) = ensure_exchange_row(client, ex).await?;
                if let Err(e) =
                    collect_funding_for_exchange_with_spec(client, id, &dbname, spec.clone()).await
                {
                    error!("funding failed for {}: {:?}", dbname, e);
                }
            }
        }
    }
    Ok(())
}
