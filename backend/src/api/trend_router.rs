use crate::api::trend::expected_returns;
use crate::api::trend::portfolio;
use crate::api::trend::regressions;
use crate::api::trend::risk;
use crate::api::trend::series;
use crate::api::trend::signals;
use crate::api::trend::volforecast;
use axum::{Router, routing::get};
use sqlx::PgPool;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/signals/xsec", get(signals::cross_section_signals))
        .route("/expected", get(expected_returns::get_expected))
        .route("/regressions/xsec", get(regressions::get_xsec_regression))
        .route("/regressions/ts", get(regressions::get_ts_regression))
        .route("/series/price", get(series::get_price_close))
        .route("/series/returns", get(series::get_returns_daily))
        .route("/series/vol", get(series::get_vol_daily_realized))
        .route("/series/volume", get(series::get_volume_ewma30))
        .route("/vol/forecast", get(volforecast::get_vol_fcst_ema))
        .route(
            "/portfolio/risk_matrix",
            get(risk::get_universe_risk_matrix),
        )
        .merge(portfolio::router())
}
