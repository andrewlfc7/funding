use axum::{
    Json,
    extract::{Query, State},
};
use sqlx::PgPool;

use super::signals::cross_section_signals; // reuse the endpoint logic
use super::types::UniverseQuery;
use super::types::{ExpectedParams, ExpectedPoint};

fn sigmoid_map(x: f64, k: f64, center: f64, cap: f64, bias: f64) -> f64 {
    // symmetric range [-cap, +cap], optional bias after mapping
    let y = -cap + (2.0 * cap) * (1.0 / (1.0 + (-k * (x - center)).exp()));
    y + bias
}

pub async fn get_expected(
    State(pool): State<PgPool>,
    Query(q): Query<ExpectedParams>,
) -> Result<Json<Vec<ExpectedPoint>>, (axum::http::StatusCode, String)> {
    // q.days likely i32 in ExpectedParams; convert to usize for UniverseQuery
    let days_i32 = q.days.unwrap_or(180);
    let days_usize: usize = days_i32.try_into().unwrap();

    // Synthesize UniverseQuery with sane defaults (ExpectedParams does not carry these fields)
    let uq = UniverseQuery {
        exchange: q.exchange.clone(),
        market_type: q.market_type.clone(),
        symbol: q.symbol.clone(),
        days: Some(days_usize),
        vol_window: Some(30),
        min_decile: Some(3),
        timeframe: Some("1d".to_string()),
    };

    let rows = cross_section_signals(State(pool.clone()), Query(uq))
        .await
        .0;
    if rows.is_empty() {
        return Ok(Json(vec![]));
    }

    // weights (you’re currently using composite already, but keep overrides in case)
    let _w_tr = q.w_trend.unwrap_or(0.30);
    let _w_mo = q.w_mom.unwrap_or(0.30);
    let _w_ew = q.w_ew.unwrap_or(0.30);
    let _w_bo = q.w_bo.unwrap_or(0.10);

    // expected return mapping
    let er_map = q.er_map.as_deref().unwrap_or("linear");
    let er_slope = q.er_slope.unwrap_or(1.0);
    let er_bias = q.er_bias.unwrap_or(0.0);

    // target exposure mapping (sigmoid)
    let k = q.sig_slope.unwrap_or(4.0);
    let center = q.sig_center.unwrap_or(0.0);
    let cap = q.sig_cap.unwrap_or(0.85);
    let bias = q.sig_bias.unwrap_or(0.0);

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let combined = r.composite;

        let expected_return = match er_map {
            "logistic" => {
                let l = 1.0;
                let x0 = 0.0;
                l / (1.0 + (-er_slope * (combined - x0)).exp()) - l / 2.0 + er_bias
            }
            _ => er_bias + er_slope * combined,
        };
        let target_exposure = sigmoid_map(combined, k, center, cap, bias);

        out.push(ExpectedPoint {
            ts: r.ts,
            symbol: r.symbol,
            combined_signal: combined,
            expected_return,
            target_exposure,
        });
    }
    Ok(Json(out))
}
