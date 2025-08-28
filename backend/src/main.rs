// src/main.rs

#![allow(clippy::let_unit_value)]

mod db;
mod utils;
mod exchanges;
mod data;

use axum::{extract::State, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use crate::utils::scheduler;


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExchangeData {
    market_symbol: String,
    funding_rate: f64,
    open_interest: f64,
    volume_24h: f64,
    funding_ts: Option<String>, // latest 8h bucket start (or legacy funding_ts)
    stats_ts:   Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TokenRow {
    token: String,
    exchanges: HashMap<String, ExchangeData>, 
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    last_updated: String,
    tokens: Vec<TokenRow>,
}

#[derive(Serialize, Debug)]
struct HealthResponse {
    ok: bool,
    tokens: usize,
    last_updated: Option<String>,
}


fn fmt_ts(ts: OffsetDateTime) -> String {
    ts.format(&Rfc3339).unwrap_or_else(|_| ts.to_string())
}

fn f64_field(v: &JsonValue, k: &str) -> f64 {
    v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0)
}

fn str_field(v: &JsonValue, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}

// ---------- Routes ----------

async fn get_funding_matrix(State(pool): State<PgPool>) -> Json<ApiResponse> {
    let rows = match sqlx::query!(
        r#"
        SELECT symbol, per_exchange, last_update
        FROM funding_matrix_view
        "#
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rs) => rs,
        Err(e) => {
            error!("query funding_matrix_view failed: {e:?}");
            return Json(ApiResponse {
                last_updated: fmt_ts(OffsetDateTime::now_utc()),
                tokens: vec![],
            });
        }
    };

    let mut tokens: Vec<TokenRow> = Vec::with_capacity(rows.len());
    let mut max_ts: Option<OffsetDateTime> = None;

    for r in rows {
        let symbol = r.symbol.unwrap_or_default();
        let mut exchanges: HashMap<String, ExchangeData> = HashMap::new();

        if let Some(JsonValue::Object(obj)) = r.per_exchange {
            for (ex_name, v) in obj {

                let market_symbol = str_field(&v, "market_symbol").unwrap_or_default();
                let funding_rate = v.get("funding_rate_8h")
                    .and_then(|x| x.as_f64())
                    .unwrap_or_else(|| f64_field(&v, "funding_rate")); // fallback

                let open_interest = f64_field(&v, "open_interest");
                let volume_24h    = f64_field(&v, "volume_24h");

                let funding_ts = str_field(&v, "funding_bucket")
                    .or_else(|| str_field(&v, "funding_ts")); // fallback
                let stats_ts   = str_field(&v, "stats_ts");

                exchanges.insert(
                    ex_name,
                    ExchangeData {
                        market_symbol,
                        funding_rate,
                        open_interest,
                        volume_24h,
                        funding_ts,
                        stats_ts,
                    },
                );
            }
        }

        let lu = r.last_update.unwrap_or_else(OffsetDateTime::now_utc);
        if max_ts.map(|m| lu > m).unwrap_or(true) {
            max_ts = Some(lu);
        }

        tokens.push(TokenRow { token: symbol, exchanges });
    }

    let last_updated = fmt_ts(max_ts.unwrap_or_else(OffsetDateTime::now_utc));
    info!("funding-matrix: {} tokens, last_updated={}", tokens.len(), last_updated);

    Json(ApiResponse { last_updated, tokens })
}

async fn health(State(pool): State<PgPool>) -> Json<HealthResponse> {
    let res = sqlx::query!(
        r#"
        SELECT COUNT(*)::BIGINT as cnt, MAX(last_update) as last_update
        FROM funding_matrix_view
        "#
    )
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => {
            let last = r.last_update.map(fmt_ts);
            Json(HealthResponse {
                ok: true,
                tokens: r.cnt.unwrap_or(0) as usize,
                last_updated: last,
            })
        }
        Err(e) => {
            error!("health query failed: {e:?}");
            Json(HealthResponse {
                ok: false,
                tokens: 0,
                last_updated: None,
            })
        }
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let pool = db::migrations::create_pool().await;

    {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            if let Err(e) = scheduler::start_scheduler(pool_clone).await {
                eprintln!("scheduler failed: {:?}", e);
            }
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/api/funding-matrix", get(get_funding_matrix))
        .route("/api/health", get(health))
        .with_state(pool)
        .layer(cors);

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    info!("Server running at http://{}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
