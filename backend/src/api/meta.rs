// src/api/meta.rs
use axum::{
    Router,
    extract::{Query, State},
    response::Json,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::error;

#[derive(Debug, Deserialize)]
pub struct OptionsQuery {
    /// "spot" or "perps"
    market_type: String,
    /// "USDT" or "USDC"
    quote: String,
}

#[derive(Debug, Serialize)]
pub struct OptionsResponse {
    pub coins: Vec<String>,
    pub exchanges: Vec<String>,
}

async fn get_trend_options(
    State(pool): State<PgPool>,
    Query(params): Query<OptionsQuery>,
) -> Json<OptionsResponse> {
    // normalize inputs
    let mt = params.market_type.to_lowercase(); // "spot" | "perps"
    let quote = params.quote.to_uppercase(); // "USDT" | "USDC"

    // Coins (base assets)
    let coins_res = sqlx::query_scalar!(
        r#"
        SELECT DISTINCT m.base_asset AS "base_asset!"
        FROM cex_markets m
        WHERE m.market_type = $1
          AND m.quote_asset = $2
          AND m.is_active = TRUE
        ORDER BY m.base_asset
        "#,
        mt,
        quote,
    )
    .fetch_all(&pool)
    .await;

    // Exchanges that have at least one active market for that filter
    let ex_res = sqlx::query_scalar!(
        r#"
        SELECT DISTINCT e.name AS "name!"
        FROM cex_exchanges e
        JOIN cex_markets m ON m.exchange_id = e.id
        WHERE e.is_active = TRUE
          AND m.is_active = TRUE
          AND m.market_type = $1
          AND m.quote_asset = $2
        ORDER BY e.name
        "#,
        mt,
        quote,
    )
    .fetch_all(&pool)
    .await;

    let (coins, exchanges) = match (coins_res, ex_res) {
        (Ok(c), Ok(e)) => (c, e),
        (Err(err_c), Err(err_e)) => {
            error!("trend/options query failed (coins & exchanges): {err_c:?} | {err_e:?}");
            (Vec::new(), Vec::new())
        }
        (Err(err_c), Ok(e)) => {
            error!("trend/options coins query failed: {err_c:?}");
            (Vec::new(), e)
        }
        (Ok(c), Err(err_e)) => {
            error!("trend/options exchanges query failed: {err_e:?}");
            (c, Vec::new())
        }
    };

    Json(OptionsResponse { coins, exchanges })
}

pub fn router() -> Router<PgPool> {
    Router::<PgPool>::new().route("/api/meta-data/options", get(get_trend_options))
}
