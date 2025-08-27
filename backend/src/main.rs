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

use crate::utils::scheduler;

// ---------- API Shapes ----------

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExchangeData {
    funding_rate: f64,
    open_interest: f64,
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

// ---------- Helpers ----------

fn fmt_ts(ts: OffsetDateTime) -> String {
    ts.format(&Rfc3339).unwrap_or_else(|_| ts.to_string())
}

// ---------- Routes ----------

async fn get_funding_matrix(State(pool): State<PgPool>) -> Json<ApiResponse> {
    let rows = match sqlx::query!(
        r#"
        SELECT symbol, rates, last_update
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

        let mut exchanges = HashMap::<String, ExchangeData>::new();
        if let Some(JsonValue::Object(obj)) = r.rates {
            for (ex, v) in obj {
                let fr = v.get("funding_rate").and_then(|x| x.as_f64()).unwrap_or(0.0);
                let oi = v.get("open_interest").and_then(|x| x.as_f64()).unwrap_or(0.0);

                if !v.get("open_interest").is_some() {
                    tracing::warn!("missing open_interest for {}/{}", symbol, ex);
                }

                exchanges.insert(ex, ExchangeData { funding_rate: fr, open_interest: oi });
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

/// GET /api/health
/// Quick connectivity & freshness check.
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

// ---------- Server Bootstrap ----------

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

    // CORS for local dev & browser apps
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
