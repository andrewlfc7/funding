use axum::{routing::get, Router};
use sqlx::PgPool;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route(
            "/api/statistics/zscore/overview",
            get(crate::api::statistics::zscore_overview::get_zscore_overview),
        )
        .route(
            "/api/statistics/volatility/analysis",
            get(crate::api::statistics::volatility_analysis::get_volatility_analysis),
        )
        .route(
            "/api/statistics/volatility/liquidity",
            get(crate::api::statistics::vol_liquidity::get_vol_liquidity),
        )
        .route(
            "/api/statistics/inter-asset/zscore",
            get(crate::api::statistics::inter_asset_zscore::get_inter_asset_zscore),
        )
        .route(
            "/api/statistics/cross-asset/analytics",
            get(crate::api::statistics::cross_asset::get_cross_asset_analytics),
            "/api/statistics/cross-asset/analytics",
            get(crate::api::statistics::cross_asset::get_cross_asset_analytics),
        )
        .route(
            "/api/statistics/cross-section/leaders-laggards",
            get(crate::api::statistics::leaders_laggards::get_leaders_laggards),
        )
        .route(
            "/api/statistics/microstructure/flow",
            get(crate::api::statistics::microstructure_flow::get_microstructure_flow),
        )

        .route(
            "/api/statistics/relative-strength/overview",
            get(crate::api::statistics::relative_strength::get_relative_strength),
        )
        .route(
            "/api/statistics/momentum/regime",
            get(crate::api::statistics::regime_momentum::get_regime_momentum),
        )

        .route(
            "/api/zscore/market-seasonality",
            axum::routing::get(crate::api::statistics::market_seasonality::get_market_seasonality),
        )
        .route(
            "/api/zscore/volatility-dynamics",
            axum::routing::get(crate::api::statistics::volatility_dynamics::get_volatility_dynamics),
        )

        .route(
            "/api/zscore/trades-analysis",
            get(crate::api::statistics::trades_analysis::get_trades_analysis),
        )

}