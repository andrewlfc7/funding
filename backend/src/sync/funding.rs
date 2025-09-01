use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use tracing::{error, info};

use crate::data::funding::{collect_funding_for_exchange_with_spec};
use super::common::lookup_exchange_id_case_insensitive;
use crate::exchanges::shared::time::TimeSpec;

pub async fn sync_funding(pool: &PgPool, exchange_opt: Option<String>, spec: TimeSpec) -> Result<()> {
    match exchange_opt {
        Some(ex) => {
            let (id, dbname) = lookup_exchange_id_case_insensitive(pool, &ex)
                .await?
                .ok_or_else(|| anyhow!("exchange not found or inactive: {}", ex))?;
            info!("funding: {} window={:?}", dbname, spec);
            collect_funding_for_exchange_with_spec(pool, id, &dbname, spec)
                .await
                .with_context(|| format!("funding sync failed for {}", dbname))?;
        }
        None => {
            info!("funding: all active exchanges, window={:?}", spec);
            let exchanges = sqlx::query!("SELECT id, name FROM exchanges WHERE is_active = true ORDER BY name")
                .fetch_all(pool)
                .await?;
            for ex in exchanges {
                if let Err(e) =
                    collect_funding_for_exchange_with_spec(pool, ex.id, &ex.name, spec.clone()).await
                {
                    error!("funding failed for {}: {:?}", ex.name, e);
                }
            }
        }
    }
    Ok(())
}