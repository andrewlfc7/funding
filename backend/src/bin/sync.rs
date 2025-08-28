
use std::env;

use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use tracing::{error, info, warn};

use backend::db::migrations;
use backend::data::coin::{refresh_all_markets, refresh_markets_for_exchange};
use backend::data::stats::collect_daily_market_stats;
use backend::data::funding::{collect_funding_for_exchange_with_spec, collect_funding_for_exchange, TimeSpec};


#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;



enum RunMode {
    Backfill, // don't SELECT markets from DB
    Normal,   // current behavior
}


async fn fetch_market_symbols_from_api(exchange_name: &str) -> anyhow::Result<Vec<String>> {
    match lower(exchange_name).as_str() {
        "paradex" => {
            use backend::exchanges::paradex::api::{client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv};
            use backend::exchanges::paradex::handler::handler::parse_paradex_markets;

            let client = ParadexClient::new(ParadexEnv::Mainnet);
            let raw = client.get_markets().await?;
            let markets = parse_paradex_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }
        "extended" => {
            use backend::exchanges::extended::api::{client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv};
            use backend::exchanges::extended::handler::handler::parse_extended_markets;

            let client = ExtendedClient::new(ExtendedEnv::Mainnet);
            let raw = client.get_markets(None).await?;
            let markets = parse_extended_markets(&raw)?;
            Ok(markets.into_iter().map(|m| m.market_symbol).collect())
        }
        other => anyhow::bail!("unsupported exchange '{}'", other),
    }
}




#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        // default to "init"
        return run_init(None).await;
    }

    let subcmd = args.remove(0).to_ascii_lowercase();
    match subcmd.as_str() {
        "init" => {
            // optional time flags
            let spec = parse_time_spec(&args)?;
            run_init(spec).await
        }

        "markets" => {
            let (exchange_opt, _spec_unused) = parse_exchange_and_spec(&args)?;
            let pool = migrations::create_pool().await;

            match exchange_opt {
                Some(ex) => {
                    let (id, dbname) = ensure_exchange_row(&pool, &ex).await?;
                    info!("markets: syncing {} (id={})", dbname, id);
                    refresh_markets_for_exchange(&pool, id, &dbname)
                        .await
                        .context("refresh_markets_for_exchange failed")?;
                }
                None => {
                    info!("markets: syncing all active exchanges");
                    refresh_all_markets(&pool)
                        .await
                        .context("refresh_all_markets failed")?;
                }
            }
            Ok(())
        }

        "stats" => {
            let (exchange_opt, _spec_unused) = parse_exchange_and_spec(&args)?;
            let pool = migrations::create_pool().await;

            match exchange_opt {
                Some(ex) => {
                    let (id, dbname) = lookup_exchange_id_case_insensitive(&pool, &ex).await?
                        .ok_or_else(|| anyhow!("exchange not found or inactive: {}", ex))?;
                    info!("stats: syncing {} (id={})", dbname, id);
                    collect_stats_for_single_exchange(&pool, id, &dbname, RunMode::Normal).await?;
                }
                None => {
                    info!("stats: syncing all active exchanges");
                    collect_daily_market_stats(&pool)
                        .await
                        .context("collect_daily_market_stats failed")?;
                }
            }
            Ok(())
        }

        "funding" => {
            let (exchange_opt, spec) = parse_exchange_and_spec(&args)?;
            let spec = spec.unwrap_or(TimeSpec::SinceLastOrLookbackHours(24));
            let pool = migrations::create_pool().await;

            match exchange_opt {
                Some(ex) => {
                    let (id, dbname) = lookup_exchange_id_case_insensitive(&pool, &ex).await?
                        .ok_or_else(|| anyhow!("exchange not found or inactive: {}", ex))?;
                    info!("funding: {} window={:?}", dbname, spec);
                    collect_funding_for_exchange_with_spec(&pool, id, &dbname, spec)
                        .await
                        .with_context(|| format!("funding sync failed for {}", dbname))?;
                }
                None => {
                    info!("funding: all active exchanges, window={:?}", spec);
                    run_funding_all(&pool, spec).await?;
                }
            }
            Ok(())
        }

        // Ensure + backfill one exchange end-to-end
        "exchange" => {
            if args.is_empty() {
                return Err(anyhow!("usage: exchange add --name <NAME> [--hours N | --between START_MS END_MS | --since-last N]"));
            }
            let sub = args[0].to_ascii_lowercase();
            if sub != "add" {
                return Err(anyhow!("unknown: exchange {}", sub));
            }
            let (name, spec) = parse_exchange_add(&args[1..])?;
            let spec = spec.unwrap_or(TimeSpec::SinceLastOrLookbackHours(24));

            let pool = migrations::create_pool().await;
            let (id, dbname) = ensure_exchange_row(&pool, &name).await?;

            info!("exchange add: {} (id={}) window={:?}", dbname, id, spec);
            refresh_markets_for_exchange(&pool, id, &dbname)
                .await
                .context("refresh_markets_for_exchange failed")?;
            collect_funding_for_exchange_with_spec(&pool, id, &dbname, spec)
                .await
                .context("funding backfill failed")?;

            collect_stats_for_single_exchange(&pool, id, &dbname, RunMode::Backfill)
                .await
                .context("stats collection failed")?;
                
            Ok(())
        }

        // Backfill one exchange end-to-end (expects already present & active)
        "backfill" => {
            let (exchange_opt, spec) = parse_exchange_and_spec(&args)?;
            let ex = exchange_opt.ok_or_else(|| anyhow!("backfill requires --exchange <name>"))?;
            let spec = spec.unwrap_or(TimeSpec::SinceLastOrLookbackHours(24));

            let pool = migrations::create_pool().await;
            let (id, dbname) = lookup_exchange_id_case_insensitive(&pool, &ex).await?
                .ok_or_else(|| anyhow!("exchange not found or inactive: {}", ex))?;

            info!("backfill: {} (id={}) window={:?}", dbname, id, spec);
            refresh_markets_for_exchange(&pool, id, &dbname)
                .await
                .context("refresh_markets_for_exchange failed")?;
            collect_funding_for_exchange_with_spec(&pool, id, &dbname, spec)
                .await
                .context("funding backfill failed")?;
            collect_stats_for_single_exchange(&pool, id, &dbname, RunMode::Backfill)
                .await
                .context("stats collection failed")?;
    
            Ok(())
        }

        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }

        other => Err(anyhow!("unknown subcommand: {}\n\n{}", other, HELP)),
    }
}

/* ----------------- Helpers (DB & runs) ----------------- */

async fn run_init(spec: Option<TimeSpec>) -> Result<()> {
    let pool = migrations::create_pool().await;
    let spec = spec.unwrap_or(TimeSpec::SinceLastOrLookbackHours(24));

    info!("init: markets (all active)");
    refresh_all_markets(&pool)
        .await
        .context("refresh_all_markets failed")?;

    info!("init: funding (all active), window={:?}", spec);
    run_funding_all(&pool, spec).await?;

    info!("init: stats (all active)");
    collect_daily_market_stats(&pool)
        .await
        .context("collect_daily_market_stats failed")?;

    info!("init: done");
    Ok(())
}

async fn collect_stats_for_single_exchange(
    pool: &PgPool,
    exchange_id: i32,
    exchange_name: &str,
    mode: RunMode, // NEW
) -> anyhow::Result<()> {
    use backend::db::insert::{insert_market_stats, insert_market_stats_by_symbol};
    use backend::exchanges::shared::types::NormalizedMarketStats;
    use tracing::{info, warn};

    match lower(exchange_name).as_str() {
        "paradex" => {
            use backend::exchanges::paradex::api::{client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv};
            use backend::exchanges::paradex::handler::handler::parse_paradex_market_stats;

            let client = ParadexClient::new(ParadexEnv::Mainnet);

            match mode {
                RunMode::Backfill => {
                    // No DB SELECT — get symbols straight from the API
                    let symbols = fetch_market_symbols_from_api(exchange_name).await?;
                    let mut owned: Vec<(String, NormalizedMarketStats)> = Vec::with_capacity(symbols.len());

                    for sym in symbols {
                        let raw = client.get_markets_summary(&sym).await?;
                        let stats = parse_paradex_market_stats(&raw)?;
                        if let Some(stat) = stats.into_iter().find(|s| s.market_symbol == sym) {
                            owned.push((sym, stat));
                        }
                    }

                    if owned.is_empty() {
                        info!("stats/backfill: no rows for {} (exchange_id={})", exchange_name, exchange_id);
                        return Ok(());
                    }

                    let borrowed: Vec<(String, &NormalizedMarketStats)> =
                        owned.iter().map(|(sym, s)| (sym.clone(), s)).collect();
                    insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                    info!("stats/backfill: inserted {} rows for {} (exchange_id={})", borrowed.len(), exchange_name, exchange_id);
                }

                RunMode::Normal => {
                    // Existing path — read markets from DB
                    let markets = sqlx::query!(
                        "SELECT id, market_symbol
                         FROM markets
                         WHERE exchange_id = $1 AND is_active = true
                         ORDER BY market_symbol",
                        exchange_id
                    )
                    .fetch_all(pool)
                    .await?;

                    let mut owned: Vec<(i32, NormalizedMarketStats)> = Vec::with_capacity(markets.len());
                    for m in markets {
                        let raw = client.get_markets_summary(&m.market_symbol).await?;
                        let stats = parse_paradex_market_stats(&raw)?;
                        if let Some(stat) = stats.into_iter().find(|s| s.market_symbol == m.market_symbol) {
                            owned.push((m.id, stat));
                        }
                    }

                    if owned.is_empty() {
                        info!("stats: no rows for {} (exchange_id={})", exchange_name, exchange_id);
                        return Ok(());
                    }

                    let borrowed: Vec<(i32, &NormalizedMarketStats)> =
                        owned.iter().map(|(mid, s)| (*mid, s)).collect();
                    insert_market_stats(pool, &borrowed).await?;
                    info!("stats: inserted {} rows for {} (exchange_id={})", borrowed.len(), exchange_name, exchange_id);
                }
            }
        }

        "extended" => {
            use backend::exchanges::extended::api::{client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv};
            use backend::exchanges::extended::handler::handler::parse_extended_market_stats;

            let client = ExtendedClient::new(ExtendedEnv::Mainnet);

            match mode {
                RunMode::Backfill => {
                    let symbols = fetch_market_symbols_from_api(exchange_name).await?;
                    let mut owned: Vec<(String, NormalizedMarketStats)> = Vec::with_capacity(symbols.len());

                    for sym in symbols {
                        let raw = client.get_market_stats(&sym).await?;
                        let stat = parse_extended_market_stats(&raw, &sym)?;
                        owned.push((sym, stat));
                    }

                    if owned.is_empty() {
                        info!("stats/backfill: no rows for {} (exchange_id={})", exchange_name, exchange_id);
                        return Ok(());
                    }

                    let borrowed: Vec<(String, &NormalizedMarketStats)> =
                        owned.iter().map(|(sym, s)| (sym.clone(), s)).collect();
                    insert_market_stats_by_symbol(pool, exchange_id, &borrowed).await?;
                    info!("stats/backfill: inserted {} rows for {} (exchange_id={})", borrowed.len(), exchange_name, exchange_id);
                }

                RunMode::Normal => {
                    let markets = sqlx::query!(
                        "SELECT id, market_symbol
                         FROM markets
                         WHERE exchange_id = $1 AND is_active = true
                         ORDER BY market_symbol",
                        exchange_id
                    )
                    .fetch_all(pool)
                    .await?;

                    let mut owned: Vec<(i32, NormalizedMarketStats)> = Vec::with_capacity(markets.len());
                    for m in markets {
                        let raw = client.get_market_stats(&m.market_symbol).await?;
                        let stat = parse_extended_market_stats(&raw, &m.market_symbol)?;
                        owned.push((m.id, stat));
                    }

                    if owned.is_empty() {
                        info!("stats: no rows for {} (exchange_id={})", exchange_name, exchange_id);
                        return Ok(());
                    }

                    let borrowed: Vec<(i32, &NormalizedMarketStats)> =
                        owned.iter().map(|(mid, s)| (*mid, s)).collect();
                    insert_market_stats(pool, &borrowed).await?;
                    info!("stats: inserted {} rows for {} (exchange_id={})", borrowed.len(), exchange_name, exchange_id);
                }
            }
        }

        _ => {
            warn!("stats: unsupported exchange '{}'", exchange_name);
        }
    }

    Ok(())
}



async fn run_funding_all(pool: &PgPool, spec: TimeSpec) -> Result<()> {
    let exchanges = sqlx::query!(
        "SELECT id, name FROM exchanges WHERE is_active = true ORDER BY name"
    )
    .fetch_all(pool)
    .await?;

    for ex in exchanges {
        if let Err(e) = collect_funding_for_exchange_with_spec(pool, ex.id, &ex.name, spec.clone()).await {
            error!("funding failed for {}: {:?}", ex.name, e);
        }
    }
    Ok(())
}

async fn lookup_exchange_id_case_insensitive(pool: &PgPool, name: &str) -> Result<Option<(i32, String)>> {
    let rec = sqlx::query!(
        r#"
        SELECT id, name
        FROM exchanges
        WHERE is_active = true AND lower(name) = lower($1)
        "#,
        name
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec.map(|r| (r.id, r.name)))
}

async fn ensure_exchange_row(pool: &PgPool, name: &str) -> Result<(i32, String)> {
    if let Some((id, dbname)) = lookup_exchange_id_case_insensitive(pool, name).await? {
        return Ok((id, dbname));
    }

    // Insert; name normalization trigger will capitalize it consistently.
    let inserted = sqlx::query!(
        r#"
        INSERT INTO exchanges (name, is_active)
        VALUES ($1, true)
        ON CONFLICT (name) DO UPDATE
            SET is_active = EXCLUDED.is_active,
                updated_at = NOW()
        RETURNING id, name
        "#,
        name
    )
    .fetch_one(pool)
    .await?;

    Ok((inserted.id, inserted.name))
}

/* ----------------- Arg parsing helpers ----------------- */

fn lower(s: &str) -> String { s.trim().to_ascii_lowercase() }

fn parse_time_spec(args: &[String]) -> Result<Option<TimeSpec>> {
    let mut i = 0;
    let mut spec: Option<TimeSpec> = None;

    while i < args.len() {
        match args[i].as_str() {
            "--hours" | "-h" => {
                if i + 1 >= args.len() { return Err(anyhow!("--hours requires a value")); }
                let hours: u64 = args[i + 1].parse().map_err(|_| anyhow!("invalid --hours value"))?;
                spec = Some(TimeSpec::LookbackHours(hours));
                i += 2;
            }
            "--between" | "-b" => {
                if i + 2 >= args.len() { return Err(anyhow!("--between requires START_MS END_MS")); }
                let start_ms: u64 = args[i + 1].parse().map_err(|_| anyhow!("invalid START_MS"))?;
                let end_ms: u64 = args[i + 2].parse().map_err(|_| anyhow!("invalid END_MS"))?;
                spec = Some(TimeSpec::Between { start_ms, end_ms });
                i += 3;
            }
            "--since-last" | "-s" => {
                if i + 1 >= args.len() { return Err(anyhow!("--since-last requires a value")); }
                let hours: u64 = args[i + 1].parse().map_err(|_| anyhow!("invalid --since-last value"))?;
                spec = Some(TimeSpec::SinceLastOrLookbackHours(hours));
                i += 2;
            }
            _other => i += 1, // ignore unrelated flags here
        }
    }

    Ok(spec)
}

fn parse_exchange_and_spec(args: &[String]) -> Result<(Option<String>, Option<TimeSpec>)> {
    let mut i = 0;
    let mut exchange: Option<String> = None;
    let mut spec: Option<TimeSpec> = None;

    while i < args.len() {
        match args[i].as_str() {
            "--exchange" | "-e" => {
                if i + 1 >= args.len() { return Err(anyhow!("--exchange requires a value")); }
                exchange = Some(args[i + 1].clone());
                i += 2;
            }
            "--name" => {
                if i + 1 >= args.len() { return Err(anyhow!("--name requires a value")); }
                exchange = Some(args[i + 1].clone());
                i += 2;
            }
            "--hours" | "-h" | "--between" | "-b" | "--since-last" | "-s" => {
                // delegate to parse_time_spec on the tail
                spec = parse_time_spec(&args[i..])?;
                break;
            }
            _ => i += 1,
        }
    }

    Ok((exchange, spec))
}

fn parse_exchange_add(args: &[String]) -> Result<(String, Option<TimeSpec>)> {
    let mut i = 0;
    let mut name: Option<String> = None;
    let mut spec: Option<TimeSpec> = None;

    while i < args.len() {
        match args[i].as_str() {
            "--name" => {
                if i + 1 >= args.len() { return Err(anyhow!("--name requires a value")); }
                name = Some(args[i + 1].clone());
                i += 2;
            }
            "--hours" | "-h" | "--between" | "-b" | "--since-last" | "-s" => {
                spec = parse_time_spec(&args[i..])?;
                break;
            }
            _ => i += 1,
        }
    }

    let name = name.ok_or_else(|| anyhow!("exchange add requires --name <NAME>"))?;
    Ok((name, spec))
}

/* ---------------------- Help text ---------------------- */

const HELP: &str = r#"sync CLI

Commands:
  init [--hours N | --between START_MS END_MS | --since-last N]
      Refresh markets, then funding, then stats for all active exchanges.
      If no time flag is provided, defaults to --since-last 24.

  markets [--exchange NAME]
      Sync markets for all active exchanges or a single exchange.

  stats [--exchange NAME]
      Collect latest market stats (OI USD + 24h volume) for all or one exchange.

  funding [--exchange NAME] [--hours N | --between START_MS END_MS | --since-last N]
      Collect funding rates over a specified window (default --since-last 24).

  exchange add --name NAME [--hours N | --between START_MS END_MS | --since-last N]
      Ensure the exchange row exists and run markets -> funding -> stats for that exchange.

  backfill --exchange NAME [--hours N | --between START_MS END_MS | --since-last N]
      Backfill one exchange end-to-end over the given window.

  help
      Show this help.
"#;

fn print_help() {
    eprintln!("{HELP}");
}
