use anyhow::Result;
use sqlx::PgPool;
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing::{info, error};
use crate::data::{coin::refresh_all_markets, stats::collect_daily_market_stats, funding::collect_funding_for_exchange};

#[derive(Debug)]
struct ExchangeCfg {
    id: i32,
    name: String,
    funding_interval_minutes: i32,
}

pub async fn start_scheduler(pool: PgPool) -> Result<()> {
    let sched = JobScheduler::new().await?;

    // --- Daily Market Discovery (midnight UTC)
    {
        let pool_clone = pool.clone();
        let job = Job::new_async("0 0 0 * * *", move |_id, _| {
            let pool = pool_clone.clone();
            Box::pin(async move {
                info!("Running daily coin discovery...");
                if let Err(e) = refresh_all_markets(&pool).await {
                    error!("Market discovery failed: {}", e);
                }
            })
        })?;
        sched.add(job).await?;
    }

    // --- Daily Market Stats Snapshot (00:30 UTC)
    {
        let pool_clone = pool.clone();
        let job = Job::new_async("0 30 0 * * *", move |_id, _| {
            let pool = pool_clone.clone();
            Box::pin(async move {
                info!("Running daily market stats snapshot...");
                if let Err(e) = collect_daily_market_stats(&pool).await {
                    error!("Collecting stats failed: {}", e);
                }
            })
        })?;
        sched.add(job).await?;
    }

    // --- Funding Jobs per Exchange ---
    let exchanges = sqlx::query!(
        r#"
        SELECT id, name, funding_interval_minutes
        FROM exchanges
        WHERE is_active = true AND funding_interval_minutes IS NOT NULL
        "#
    )
    .fetch_all(&pool)
    .await?;

    for exch in exchanges {
        let cfg = ExchangeCfg {
            id: exch.id,
            name: exch.name.clone(),
            funding_interval_minutes: exch.funding_interval_minutes.unwrap(),
        };

        let cron_expr = minutes_to_cron(cfg.funding_interval_minutes)?;
        info!(
            "Scheduling funding collection for {} (interval={}m) → {}",
            cfg.name, cfg.funding_interval_minutes, cron_expr
        );

        let pool_clone = pool.clone();
        let exch_name = cfg.name.clone();
        let exch_id = cfg.id;

        let job = Job::new_async(&cron_expr, move |_id, _| {
            let pool = pool_clone.clone();
            let name = exch_name.clone();
            Box::pin(async move {
                info!("Running funding collection for {}", name);
                if let Err(e) = collect_funding_for_exchange(&pool, exch_id, &name).await {
                    error!("Funding collection {} failed: {}", name, e);
                }
            })
        })?;
        sched.add(job).await?;
    }

    sched.start().await?;
    Ok(())
}



fn minutes_to_cron(minutes: i32) -> Result<String> {
    match minutes {
        60 => Ok("0 0 * * * *".to_string()),        // hourly
        480 => Ok("0 0 */8 * * *".to_string()),     // every 8 hours
        m if m < 60 => Ok(format!("0 */{} * * * *", m)),
        m if m % 60 == 0 => Ok(format!("0 0 */{} * * *", m / 60)),
        m => Err(anyhow::anyhow!("Unsupported funding interval: {}", m)),
    }
}