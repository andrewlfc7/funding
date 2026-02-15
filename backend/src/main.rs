#![allow(clippy::let_unit_value)]

use ::clickhouse::{Client, Row};
use anyhow::Result;
use axum::{
    Router,
    extract::{Query, State},
    response::Json,
    routing::get,
};
use backend::db::{clickhouse as chdb, migrations};
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExchangeData {
    market_symbol: String,
    funding_rate: f64,
    open_interest: f64,
    volume_24h: f64,
    funding_ts: Option<String>,
    stats_ts: Option<String>,
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

#[derive(Debug, Deserialize)]
struct MetaOptionsQuery {
    #[serde(default = "default_market_type", alias = "marketType")]
    market_type: String,
    #[serde(default = "default_quote")]
    quote: String,
}

#[derive(Debug, Serialize)]
struct OptionsResponse {
    coins: Vec<String>,
    exchanges: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TrendQuery {
    exchange: String,
    #[serde(default = "default_market_type", alias = "marketType")]
    market_type: String,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    days: Option<u32>,
    #[serde(default)]
    span: Option<usize>,
    #[serde(default)]
    annualize: Option<bool>,
    #[serde(default, alias = "topN")]
    top_n: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CrossAssetQuery {
    #[serde(default)]
    index: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RelativeStrengthQuery {
    #[serde(default)]
    base: Option<String>,
    #[serde(default, alias = "baseCoin")]
    base_coin: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct AnalyticsQuery {
    #[serde(default = "default_exchange")]
    exchange: String,
    #[serde(default = "default_market_type", alias = "marketType")]
    market_type: String,
    #[serde(default)]
    timeframe: Option<String>,
    #[serde(default)]
    period: Option<String>,
    #[serde(default, alias = "topN")]
    top_n: Option<usize>,
    #[serde(default)]
    coin: Option<String>,
    #[serde(default)]
    base: Option<String>,
    #[serde(default, alias = "baseCoin")]
    base_coin: Option<String>,
    #[serde(default, alias = "compareCoin")]
    compare_coin: Option<String>,
    #[serde(default)]
    index: Option<String>,
    #[serde(default, alias = "indexCoin")]
    index_coin: Option<String>,
    #[serde(default)]
    coins: Option<String>,
    #[serde(default, alias = "compareCoins")]
    compare_coins: Option<String>,
    #[serde(default)]
    window: Option<usize>,
    #[serde(default, alias = "volWindow")]
    vol_window: Option<usize>,
    #[serde(default, alias = "vovWindow")]
    vov_window: Option<usize>,
    #[serde(default, alias = "momentWindow")]
    moment_window: Option<usize>,
    #[serde(default, alias = "histogramCoin")]
    histogram_coin: Option<String>,
    #[serde(default, alias = "binStepPct")]
    bin_step_pct: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TradesAnalysisQuery {
    #[serde(default)]
    assets: Option<String>,
    #[serde(default)]
    interval: Option<String>,
    #[serde(default)]
    period: Option<String>,
    #[serde(default, alias = "distributionAsset")]
    distribution_asset: Option<String>,
}

#[derive(Debug, Serialize)]
struct TrendSignalPoint {
    ts: i64,
    symbol: String,
    trend: f64,
    momentum: f64,
    ewmac: f64,
    breakout: f64,
    composite: f64,
}

#[derive(Debug, Serialize)]
struct TimeSeriesPoint {
    date: String,
    value: f64,
}

#[derive(Debug, Serialize)]
struct VolumeSeriesPoint {
    date: String,
    volume_ewma: f64,
    dollar_volume_ewma: f64,
}

#[derive(Debug, Row, Deserialize)]
struct IdRow {
    id: i32,
}

#[derive(Debug, Row, Deserialize)]
struct CoinNameRow {
    symbol: String,
}

#[derive(Debug, Row, Deserialize)]
struct ExchangeNameRow {
    name: String,
}

#[derive(Debug, Row, Deserialize, Clone)]
struct KlineHourRow {
    time_ms: i64,
    close_px: f64,
    high_px: f64,
    low_px: f64,
    volume: f64,
}

#[derive(Debug, Row, Deserialize)]
struct TopMarketRow {
    symbol: String,
    market_id: i32,
}

#[derive(Debug, Row, Deserialize)]
struct TopMarketVolRow {
    symbol: String,
    market_id: i32,
    usd_volume: f64,
}

#[derive(Debug, Row, Deserialize, Clone)]
struct TradeRow {
    trade_id: String,
    trade_time_ms: i64,
    side: String,
    price: f64,
    qty: f64,
    quote_qty: f64,
}

#[derive(Debug, Clone)]
struct MarketRef {
    symbol: String,
    market_id: i32,
    usd_volume: f64,
}

#[derive(Debug, Clone)]
struct SymbolSeries {
    symbol: String,
    market_id: i32,
    bars: Vec<KlineHourRow>,
}

fn default_market_type() -> String {
    "spot".to_string()
}

fn default_quote() -> String {
    "USDT".to_string()
}

fn default_exchange() -> String {
    "binance".to_string()
}

fn normalize_market_type(input: &str) -> String {
    match input.trim().to_ascii_lowercase().as_str() {
        "perp" | "perps" | "perpetual" => "perps".to_string(),
        _ => "spot".to_string(),
    }
}

fn normalize_symbol(input: Option<String>) -> String {
    input
        .unwrap_or_else(|| "BTC".to_string())
        .trim()
        .to_ascii_uppercase()
}

fn days_or_default(days: Option<u32>) -> u32 {
    days.unwrap_or(30).clamp(1, 365)
}

fn parse_period_days(period: Option<&str>, default_days: u32) -> u32 {
    let Some(raw) = period else {
        return default_days.clamp(1, 365);
    };
    let p = raw.trim().to_ascii_lowercase();

    if let Some(num) = p.strip_suffix('d').and_then(|v| v.parse::<u32>().ok()) {
        return num.clamp(1, 365);
    }
    if let Some(num) = p.strip_suffix('h').and_then(|v| v.parse::<u32>().ok()) {
        let days = ((num as f64) / 24.0).ceil() as u32;
        return days.clamp(1, 365);
    }
    if let Ok(num) = p.parse::<u32>() {
        return num.clamp(1, 365);
    }

    default_days.clamp(1, 365)
}

fn parse_period_hours(period: Option<&str>, default_hours: u32) -> u32 {
    let Some(raw) = period else {
        return default_hours.max(1);
    };
    let p = raw.trim().to_ascii_lowercase();
    if let Some(num) = p.strip_suffix('h').and_then(|v| v.parse::<u32>().ok()) {
        return num.max(1);
    }
    if let Some(num) = p.strip_suffix('d').and_then(|v| v.parse::<u32>().ok()) {
        return num.saturating_mul(24).max(1);
    }
    if let Ok(num) = p.parse::<u32>() {
        return num.max(1);
    }
    default_hours.max(1)
}

fn timeframe_hours(tf: Option<&str>) -> usize {
    match tf.unwrap_or("1h").trim().to_ascii_lowercase().as_str() {
        "4h" => 4,
        "1d" | "d1" => 24,
        _ => 1,
    }
}

fn parse_csv_symbols(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn fmt_ts_ms(ts_ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ts_ms).map(|dt| dt.to_rfc3339())
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn finite(x: f64) -> f64 {
    if x.is_finite() { x } else { 0.0 }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

fn pct_returns(prices: &[f64]) -> Vec<f64> {
    if prices.is_empty() {
        return vec![];
    }

    let mut out = Vec::with_capacity(prices.len());
    out.push(0.0);

    for pair in prices.windows(2) {
        let p0 = pair[0];
        let p1 = pair[1];
        out.push(if p0 > 0.0 { (p1 / p0) - 1.0 } else { 0.0 });
    }

    out
}

fn rolling_std(values: &[f64], window: usize) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }

    let window = window.max(2);
    let mut out = vec![0.0; n];

    for i in 0..n {
        if i + 1 < window {
            continue;
        }

        let slice = &values[i + 1 - window..=i];
        let m = mean(slice);
        let var = slice
            .iter()
            .map(|v| {
                let d = *v - m;
                d * d
            })
            .sum::<f64>()
            / (slice.len() as f64);
        out[i] = var.sqrt();
    }

    out
}

fn ema(values: &[f64], span: usize) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }

    let span = span.max(1) as f64;
    let alpha = 2.0 / (span + 1.0);

    let mut out = Vec::with_capacity(values.len());
    let mut s = values[0];
    out.push(s);

    for &x in &values[1..] {
        s = alpha * x + (1.0 - alpha) * s;
        out.push(s);
    }

    out
}

fn log_returns(prices: &[f64]) -> Vec<f64> {
    if prices.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(prices.len());
    out.push(0.0);
    for pair in prices.windows(2) {
        let p0 = pair[0];
        let p1 = pair[1];
        let r = if p0 > 0.0 && p1 > 0.0 {
            (p1 / p0).ln()
        } else {
            0.0
        };
        out.push(finite(r));
    }
    out
}

fn rolling_mean(values: &[f64], window: usize) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }
    let window = window.max(1);
    let mut out = vec![0.0; n];
    let mut sum = 0.0;
    for i in 0..n {
        sum += values[i];
        if i >= window {
            sum -= values[i - window];
        }
        let denom = if i + 1 < window { i + 1 } else { window };
        out[i] = sum / denom as f64;
    }
    out
}

fn zscore_series(values: &[f64], window: usize) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }
    let window = window.max(2);
    let means = rolling_mean(values, window);
    let stds = rolling_std(values, window);
    let mut out = vec![0.0; n];
    for i in 0..n {
        let sd = stds[i];
        out[i] = if sd.is_finite() && sd > 1e-12 {
            (values[i] - means[i]) / sd
        } else {
            0.0
        };
        out[i] = finite(out[i]);
    }
    out
}

fn minmax(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 1.0);
    }
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for &v in values {
        if v.is_finite() {
            mn = mn.min(v);
            mx = mx.max(v);
        }
    }
    if !mn.is_finite() || !mx.is_finite() || mx <= mn {
        (0.0, 1.0)
    } else {
        (mn, mx)
    }
}

fn covariance(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }
    let xs = &x[x.len() - n..];
    let ys = &y[y.len() - n..];
    let mx = mean(xs);
    let my = mean(ys);
    let mut acc = 0.0;
    for i in 0..n {
        acc += (xs[i] - mx) * (ys[i] - my);
    }
    acc / n as f64
}

fn variance(x: &[f64]) -> f64 {
    covariance(x, x).max(0.0)
}

fn correlation(x: &[f64], y: &[f64]) -> f64 {
    let cov = covariance(x, y);
    let sx = variance(x).sqrt();
    let sy = variance(y).sqrt();
    if sx > 1e-12 && sy > 1e-12 {
        finite(cov / (sx * sy))
    } else {
        0.0
    }
}

fn beta(x: &[f64], y: &[f64]) -> f64 {
    let var_y = variance(y);
    if var_y > 1e-12 {
        finite(covariance(x, y) / var_y)
    } else {
        0.0
    }
}

fn r2(x: &[f64], y: &[f64]) -> f64 {
    let c = correlation(x, y);
    finite(c * c)
}

fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .collect::<Vec<_>>();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let idx = ((v.len() - 1) as f64 * p.clamp(0.0, 1.0)).round() as usize;
    v[idx]
}

fn autocorr_lag1(values: &[f64]) -> f64 {
    if values.len() < 3 {
        return 0.0;
    }
    let x = &values[1..];
    let y = &values[..values.len() - 1];
    correlation(x, y)
}

fn skewness(values: &[f64]) -> f64 {
    if values.len() < 3 {
        return 0.0;
    }
    let m = mean(values);
    let mut m2 = 0.0;
    let mut m3 = 0.0;
    for &v in values {
        let d = v - m;
        m2 += d * d;
        m3 += d * d * d;
    }
    let n = values.len() as f64;
    let var = m2 / n;
    if var <= 1e-12 {
        0.0
    } else {
        finite((m3 / n) / var.powf(1.5))
    }
}

fn kurtosis(values: &[f64]) -> f64 {
    if values.len() < 4 {
        return 0.0;
    }
    let m = mean(values);
    let mut m2 = 0.0;
    let mut m4 = 0.0;
    for &v in values {
        let d = v - m;
        let d2 = d * d;
        m2 += d2;
        m4 += d2 * d2;
    }
    let n = values.len() as f64;
    let var = m2 / n;
    if var <= 1e-12 {
        0.0
    } else {
        finite((m4 / n) / (var * var) - 3.0)
    }
}

fn histogram(values: &[f64], bins: usize, min_v: f64, max_v: f64) -> (Vec<f64>, Vec<usize>) {
    if bins == 0 {
        return (vec![], vec![]);
    }
    let min_v = if min_v.is_finite() { min_v } else { 0.0 };
    let max_v = if max_v.is_finite() && max_v > min_v {
        max_v
    } else {
        min_v + 1.0
    };
    let width = (max_v - min_v) / bins as f64;
    let mut buckets = Vec::with_capacity(bins);
    let mut counts = vec![0usize; bins];
    for i in 0..bins {
        buckets.push(min_v + i as f64 * width);
    }
    for &v in values {
        if !v.is_finite() {
            continue;
        }
        let mut idx = ((v - min_v) / width).floor() as isize;
        if idx < 0 {
            idx = 0;
        }
        if idx as usize >= bins {
            idx = bins as isize - 1;
        }
        counts[idx as usize] += 1;
    }
    (buckets, counts)
}

fn rolling_sum(values: &[f64], window: usize) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }
    let window = window.max(1);
    let mut out = vec![0.0; n];
    let mut sum = 0.0;
    for i in 0..n {
        sum += values[i];
        if i >= window {
            sum -= values[i - window];
        }
        out[i] = sum;
    }
    out
}

fn lagged_correlation(x: &[f64], y: &[f64], lag: i32) -> f64 {
    let n = x.len().min(y.len());
    if n < 8 {
        return 0.0;
    }
    let mut a = Vec::new();
    let mut b = Vec::new();
    if lag >= 0 {
        let l = lag as usize;
        for i in l..n {
            a.push(x[i]);
            b.push(y[i - l]);
        }
    } else {
        let l = (-lag) as usize;
        for i in l..n {
            a.push(x[i - l]);
            b.push(y[i]);
        }
    }
    correlation(&a, &b)
}

fn resample_bars(bars: &[KlineHourRow], step_hours: usize) -> Vec<KlineHourRow> {
    if step_hours <= 1 || bars.is_empty() {
        return bars.to_vec();
    }

    let bucket_ms = (step_hours as i64) * 60 * 60 * 1000;
    let mut out = Vec::new();

    let mut cur_bucket = bars[0].time_ms - (bars[0].time_ms % bucket_ms);
    let mut close = bars[0].close_px;
    let mut high = bars[0].high_px;
    let mut low = bars[0].low_px;
    let mut volume = 0.0;

    for bar in bars {
        let b = bar.time_ms - (bar.time_ms % bucket_ms);
        if b != cur_bucket {
            out.push(KlineHourRow {
                time_ms: cur_bucket,
                close_px: close,
                high_px: high,
                low_px: low,
                volume,
            });
            cur_bucket = b;
            high = bar.high_px;
            low = bar.low_px;
            volume = 0.0;
        }
        close = bar.close_px;
        high = high.max(bar.high_px);
        low = low.min(bar.low_px);
        volume += bar.volume;
    }

    out.push(KlineHourRow {
        time_ms: cur_bucket,
        close_px: close,
        high_px: high,
        low_px: low,
        volume,
    });

    out
}

async fn resolve_market_id(
    client: &Client,
    exchange: &str,
    market_type: &str,
    symbol: &str,
) -> Result<Option<i32>> {
    let rows = client
        .query(
            r#"
            SELECT m.id AS id
            FROM cex_exchanges AS e FINAL
            INNER JOIN cex_markets AS m FINAL ON m.exchange_id = e.id
            WHERE lower(e.name) = lower(?)
              AND m.market_type = ?
              AND upper(m.symbol) = upper(?)
              AND m.is_active = 1
            ORDER BY multiIf(
                m.quote_asset = 'USDT', 0,
                m.quote_asset = 'USD', 1,
                m.quote_asset = 'USDC', 2,
                3
            ), m.id
            LIMIT 1
            "#,
        )
        .bind(exchange)
        .bind(market_type)
        .bind(symbol)
        .fetch_all::<IdRow>()
        .await?;

    Ok(rows.into_iter().next().map(|r| r.id))
}

async fn fetch_hourly_klines(
    client: &Client,
    market_id: i32,
    days: u32,
) -> Result<Vec<KlineHourRow>> {
    let since_ms = Utc::now().timestamp_millis() - (days as i64) * 24 * 60 * 60 * 1000;

    let rows = client
        .query(
            r#"
            SELECT time_ms, close_px, high_px, low_px, volume
            FROM klines_hourly
            WHERE market_id = ? AND time_ms >= ?
            ORDER BY time_ms
            "#,
        )
        .bind(market_id)
        .bind(since_ms)
        .fetch_all::<KlineHourRow>()
        .await?;

    Ok(rows)
}

async fn fetch_top_markets_by_volume(
    client: &Client,
    exchange: &str,
    market_type: &str,
    days: u32,
    top_n: usize,
) -> Result<Vec<MarketRef>> {
    let since_ms = Utc::now().timestamp_millis() - (days as i64) * 24 * 60 * 60 * 1000;
    let limit = top_n.clamp(1, 200) as u64;

    let rows = client
        .query(
            r#"
            SELECT
                symbol,
                argMax(market_id, vol) AS market_id,
                max(vol) AS usd_volume
            FROM (
                SELECT
                    m.symbol AS symbol,
                    m.id AS market_id,
                    sum(k.close_px * k.volume) AS vol
                FROM cex_exchanges AS e FINAL
                INNER JOIN cex_markets AS m FINAL ON m.exchange_id = e.id
                INNER JOIN klines_hourly AS k ON k.market_id = m.id
                WHERE lower(e.name) = lower(?)
                  AND m.market_type = ?
                  AND m.is_active = 1
                  AND k.time_ms >= ?
                GROUP BY m.symbol, m.id
            )
            GROUP BY symbol
            ORDER BY usd_volume DESC
            LIMIT ?
            "#,
        )
        .bind(exchange)
        .bind(market_type)
        .bind(since_ms)
        .bind(limit)
        .fetch_all::<TopMarketVolRow>()
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| MarketRef {
            symbol: r.symbol,
            market_id: r.market_id,
            usd_volume: r.usd_volume,
        })
        .collect())
}

async fn fetch_trade_rows(client: &Client, market_id: i32, since_ms: i64) -> Result<Vec<TradeRow>> {
    let rows = client
        .query(
            r#"
            SELECT trade_id, trade_time_ms, side, price, qty, quote_qty
            FROM trades
            WHERE market_id = ? AND trade_time_ms >= ?
            ORDER BY trade_time_ms
            "#,
        )
        .bind(market_id)
        .bind(since_ms)
        .fetch_all::<TradeRow>()
        .await?;
    Ok(rows)
}

async fn build_universe_refs(
    client: &Client,
    q: &AnalyticsQuery,
    default_top_n: usize,
) -> Result<Vec<MarketRef>> {
    let exchange = q.exchange.trim().to_string();
    let market_type = normalize_market_type(&q.market_type);
    let days = parse_period_days(q.period.as_deref(), 30);
    let top_n = q.top_n.unwrap_or(default_top_n).clamp(1, 200);

    let mut symbols = HashSet::new();
    for s in parse_csv_symbols(q.coins.as_deref()) {
        symbols.insert(s);
    }
    for s in parse_csv_symbols(q.compare_coins.as_deref()) {
        symbols.insert(s);
    }
    for s in [
        q.coin.clone(),
        q.base.clone(),
        q.base_coin.clone(),
        q.compare_coin.clone(),
        q.index.clone(),
        q.index_coin.clone(),
    ]
    .into_iter()
    .flatten()
    {
        let sym = s.trim().to_ascii_uppercase();
        if !sym.is_empty() {
            symbols.insert(sym);
        }
    }

    if symbols.is_empty() {
        return fetch_top_markets_by_volume(client, &exchange, &market_type, days, top_n).await;
    }

    let mut out = Vec::new();
    for sym in symbols {
        if let Some(market_id) = resolve_market_id(client, &exchange, &market_type, &sym).await? {
            out.push(MarketRef {
                symbol: sym,
                market_id,
                usd_volume: 0.0,
            });
        }
    }
    Ok(out)
}

async fn load_symbol_series(
    client: &Client,
    refs: &[MarketRef],
    days: u32,
    timeframe_h: usize,
) -> Vec<SymbolSeries> {
    let mut out = Vec::new();
    for r in refs {
        match fetch_hourly_klines(client, r.market_id, days).await {
            Ok(bars) => {
                let bars = resample_bars(&bars, timeframe_h);
                if bars.len() >= 8 {
                    out.push(SymbolSeries {
                        symbol: r.symbol.clone(),
                        market_id: r.market_id,
                        bars,
                    });
                }
            }
            Err(e) => warn!(
                "failed loading klines for {} (market_id={}): {e:?}",
                r.symbol, r.market_id
            ),
        }
    }
    out
}

fn date_str_from_ms(ts_ms: i64) -> String {
    fmt_ts_ms(ts_ms).unwrap_or_else(|| "1970-01-01T00:00:00+00:00".to_string())
}

fn build_signal_point(symbol: &str, bars: &[KlineHourRow]) -> Option<TrendSignalPoint> {
    if bars.len() < 8 {
        return None;
    }

    let prices: Vec<f64> = bars.iter().map(|b| b.close_px).collect();
    let returns = pct_returns(&prices);

    let n = prices.len();
    let trend_lb = (n - 1).min(24);
    let trend = if trend_lb > 0 {
        let p0 = prices[n - 1 - trend_lb];
        if p0 > 0.0 {
            (prices[n - 1] / p0) - 1.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    let mom_win = returns.len().min(12).max(1);
    let momentum = mean(&returns[returns.len() - mom_win..]);

    let fast_win = n.min(8);
    let slow_win = n.min(24);
    let fast = mean(&prices[n - fast_win..]);
    let slow = mean(&prices[n - slow_win..]);
    let ewmac = if slow.abs() > 1e-12 {
        (fast - slow) / slow
    } else {
        0.0
    };

    let breakout_win = n.min(24);
    let recent = &prices[n - breakout_win..];
    let hi = recent.iter().fold(f64::NEG_INFINITY, |acc, v| acc.max(*v));
    let lo = recent.iter().fold(f64::INFINITY, |acc, v| acc.min(*v));
    let breakout = if hi > lo {
        ((prices[n - 1] - lo) / (hi - lo)) * 2.0 - 1.0
    } else {
        0.0
    };

    let composite = (trend + momentum + ewmac + breakout) / 4.0;

    Some(TrendSignalPoint {
        ts: bars[n - 1].time_ms / 1000,
        symbol: symbol.to_string(),
        trend: finite(trend),
        momentum: finite(momentum),
        ewmac: finite(ewmac),
        breakout: finite(breakout),
        composite: finite(composite),
    })
}

async fn get_funding_matrix(State(client): State<Client>) -> Json<ApiResponse> {
    let rows = match chdb::funding_matrix_rows(&client).await {
        Ok(rs) => rs,
        Err(e) => {
            error!("query funding matrix failed: {e:?}");
            return Json(ApiResponse {
                last_updated: now_rfc3339(),
                tokens: vec![],
            });
        }
    };

    let mut by_token: BTreeMap<String, HashMap<String, ExchangeData>> = BTreeMap::new();
    let mut max_ts_ms: Option<i64> = None;

    for r in rows {
        let exchanges = by_token.entry(r.token).or_default();

        let funding_ts = r.funding_ts_ms.and_then(fmt_ts_ms);
        let stats_ts = r.stats_ts_ms.and_then(fmt_ts_ms);

        if let Some(ts) = r.funding_ts_ms {
            if max_ts_ms.map(|m| ts > m).unwrap_or(true) {
                max_ts_ms = Some(ts);
            }
        }
        if let Some(ts) = r.stats_ts_ms {
            if max_ts_ms.map(|m| ts > m).unwrap_or(true) {
                max_ts_ms = Some(ts);
            }
        }

        exchanges.insert(
            r.exchange,
            ExchangeData {
                market_symbol: r.market_symbol,
                funding_rate: r.funding_rate.unwrap_or(0.0),
                open_interest: r.open_interest.unwrap_or(0.0),
                volume_24h: r.volume_24h.unwrap_or(0.0),
                funding_ts,
                stats_ts,
            },
        );
    }

    let tokens: Vec<TokenRow> = by_token
        .into_iter()
        .map(|(token, exchanges)| TokenRow { token, exchanges })
        .collect();

    let last_updated = max_ts_ms.and_then(fmt_ts_ms).unwrap_or_else(now_rfc3339);

    info!(
        "funding-matrix: {} tokens, last_updated={}",
        tokens.len(),
        last_updated
    );

    Json(ApiResponse {
        last_updated,
        tokens,
    })
}

async fn health(State(client): State<Client>) -> Json<HealthResponse> {
    let count = chdb::funding_matrix_token_count(&client).await;
    let rows = chdb::funding_matrix_rows(&client).await;

    match (count, rows) {
        (Ok(cnt), Ok(rs)) => {
            let mut last_updated_ms: Option<i64> = None;
            for r in rs {
                if let Some(ts) = r.funding_ts_ms {
                    if last_updated_ms.map(|m| ts > m).unwrap_or(true) {
                        last_updated_ms = Some(ts);
                    }
                }
                if let Some(ts) = r.stats_ts_ms {
                    if last_updated_ms.map(|m| ts > m).unwrap_or(true) {
                        last_updated_ms = Some(ts);
                    }
                }
            }

            Json(HealthResponse {
                ok: true,
                tokens: cnt as usize,
                last_updated: last_updated_ms.and_then(fmt_ts_ms),
            })
        }
        (Err(e1), Err(e2)) => {
            error!("health query failed: {e1:?} | {e2:?}");
            Json(HealthResponse {
                ok: false,
                tokens: 0,
                last_updated: None,
            })
        }
        (Err(e), _) | (_, Err(e)) => {
            error!("health query failed: {e:?}");
            Json(HealthResponse {
                ok: false,
                tokens: 0,
                last_updated: None,
            })
        }
    }
}

async fn get_meta_options(
    State(client): State<Client>,
    Query(q): Query<MetaOptionsQuery>,
) -> Json<OptionsResponse> {
    let market_type = normalize_market_type(&q.market_type);
    let quote = q.quote.trim().to_ascii_uppercase();

    let coins_res = client
        .query(
            r#"
            SELECT DISTINCT symbol
            FROM cex_markets AS m FINAL
            WHERE m.market_type = ?
              AND upper(m.quote_asset) = ?
              AND m.is_active = 1
            ORDER BY symbol
            "#,
        )
        .bind(market_type.clone())
        .bind(quote.clone())
        .fetch_all::<CoinNameRow>()
        .await;

    let exchanges_res = client
        .query(
            r#"
            SELECT DISTINCT e.name AS name
            FROM cex_exchanges AS e FINAL
            INNER JOIN cex_markets AS m FINAL ON m.exchange_id = e.id
            WHERE e.is_active = 1
              AND m.is_active = 1
              AND m.market_type = ?
              AND upper(m.quote_asset) = ?
            ORDER BY name
            "#,
        )
        .bind(market_type)
        .bind(quote)
        .fetch_all::<ExchangeNameRow>()
        .await;

    match (coins_res, exchanges_res) {
        (Ok(coins_rows), Ok(exchange_rows)) => Json(OptionsResponse {
            coins: coins_rows.into_iter().map(|r| r.symbol).collect(),
            exchanges: exchange_rows.into_iter().map(|r| r.name).collect(),
        }),
        (c, e) => {
            warn!(
                "meta options query failed: coins_ok={} exchanges_ok={}",
                c.is_ok(),
                e.is_ok()
            );
            Json(OptionsResponse {
                coins: vec![],
                exchanges: vec![],
            })
        }
    }
}

async fn get_trend_series_price(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Vec<TimeSeriesPoint>> {
    let symbol = normalize_symbol(q.symbol);
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);

    let market_id = match resolve_market_id(&client, &q.exchange, &market_type, &symbol).await {
        Ok(Some(id)) => id,
        Ok(None) => return Json(vec![]),
        Err(e) => {
            error!("resolve market failed for price series: {e:?}");
            return Json(vec![]);
        }
    };

    let bars = match fetch_hourly_klines(&client, market_id, days).await {
        Ok(v) => v,
        Err(e) => {
            error!("fetch price series failed: {e:?}");
            return Json(vec![]);
        }
    };

    Json(
        bars.into_iter()
            .map(|b| TimeSeriesPoint {
                date: date_str_from_ms(b.time_ms),
                value: finite(b.close_px),
            })
            .collect(),
    )
}

async fn get_trend_series_returns(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Vec<TimeSeriesPoint>> {
    let symbol = normalize_symbol(q.symbol);
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);

    let market_id = match resolve_market_id(&client, &q.exchange, &market_type, &symbol).await {
        Ok(Some(id)) => id,
        Ok(None) => return Json(vec![]),
        Err(e) => {
            error!("resolve market failed for returns series: {e:?}");
            return Json(vec![]);
        }
    };

    let bars = match fetch_hourly_klines(&client, market_id, days).await {
        Ok(v) => v,
        Err(e) => {
            error!("fetch returns series failed: {e:?}");
            return Json(vec![]);
        }
    };

    let prices: Vec<f64> = bars.iter().map(|b| b.close_px).collect();
    let returns = pct_returns(&prices);

    Json(
        bars.into_iter()
            .zip(returns.into_iter())
            .map(|(b, r)| TimeSeriesPoint {
                date: date_str_from_ms(b.time_ms),
                value: finite(r),
            })
            .collect(),
    )
}

async fn get_trend_series_vol(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Vec<TimeSeriesPoint>> {
    let symbol = normalize_symbol(q.symbol);
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let annualize = q.annualize.unwrap_or(false);

    let market_id = match resolve_market_id(&client, &q.exchange, &market_type, &symbol).await {
        Ok(Some(id)) => id,
        Ok(None) => return Json(vec![]),
        Err(e) => {
            error!("resolve market failed for vol series: {e:?}");
            return Json(vec![]);
        }
    };

    let bars = match fetch_hourly_klines(&client, market_id, days).await {
        Ok(v) => v,
        Err(e) => {
            error!("fetch vol series failed: {e:?}");
            return Json(vec![]);
        }
    };

    let prices: Vec<f64> = bars.iter().map(|b| b.close_px).collect();
    let returns = pct_returns(&prices);
    let mut vol = rolling_std(&returns, 24);

    if annualize {
        let factor = (24.0_f64 * 365.0_f64).sqrt();
        for v in &mut vol {
            *v *= factor;
        }
    }

    Json(
        bars.into_iter()
            .zip(vol.into_iter())
            .map(|(b, v)| TimeSeriesPoint {
                date: date_str_from_ms(b.time_ms),
                value: finite(v),
            })
            .collect(),
    )
}

async fn get_trend_series_volume(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Vec<VolumeSeriesPoint>> {
    let symbol = normalize_symbol(q.symbol);
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let span = q.span.unwrap_or(60).clamp(2, 500);

    let market_id = match resolve_market_id(&client, &q.exchange, &market_type, &symbol).await {
        Ok(Some(id)) => id,
        Ok(None) => return Json(vec![]),
        Err(e) => {
            error!("resolve market failed for volume series: {e:?}");
            return Json(vec![]);
        }
    };

    let bars = match fetch_hourly_klines(&client, market_id, days).await {
        Ok(v) => v,
        Err(e) => {
            error!("fetch volume series failed: {e:?}");
            return Json(vec![]);
        }
    };

    let volumes: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    let dollar_volumes: Vec<f64> = bars.iter().map(|b| b.volume * b.close_px).collect();

    let volume_ewma = ema(&volumes, span);
    let dollar_ewma = ema(&dollar_volumes, span);

    Json(
        bars.into_iter()
            .zip(volume_ewma.into_iter().zip(dollar_ewma.into_iter()))
            .map(|(b, (v_ewma, d_ewma))| VolumeSeriesPoint {
                date: date_str_from_ms(b.time_ms),
                volume_ewma: finite(v_ewma),
                dollar_volume_ewma: finite(d_ewma),
            })
            .collect(),
    )
}

async fn get_trend_signals_xsec(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Vec<TrendSignalPoint>> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);

    if let Some(symbol) = q.symbol {
        let symbol = symbol.trim().to_ascii_uppercase();
        let market_id = match resolve_market_id(&client, &q.exchange, &market_type, &symbol).await {
            Ok(Some(id)) => id,
            Ok(None) => return Json(vec![]),
            Err(e) => {
                error!("resolve market failed for trend signals: {e:?}");
                return Json(vec![]);
            }
        };

        let bars = match fetch_hourly_klines(&client, market_id, days).await {
            Ok(v) => v,
            Err(e) => {
                error!("fetch bars failed for trend signals: {e:?}");
                return Json(vec![]);
            }
        };

        return Json(build_signal_point(&symbol, &bars).into_iter().collect());
    }

    let since_ms = Utc::now().timestamp_millis() - (days as i64) * 24 * 60 * 60 * 1000;
    let top_n = q.top_n.unwrap_or(20).clamp(1, 100) as u64;

    let top_markets = match client
        .query(
            r#"
            SELECT
                m.symbol AS symbol,
                m.id AS market_id
            FROM cex_exchanges AS e FINAL
            INNER JOIN cex_markets AS m FINAL ON m.exchange_id = e.id
            INNER JOIN klines_hourly AS k ON k.market_id = m.id
            WHERE lower(e.name) = lower(?)
              AND m.market_type = ?
              AND m.is_active = 1
              AND k.time_ms >= ?
            GROUP BY m.symbol, m.id
            ORDER BY sum(k.close_px * k.volume) DESC
            LIMIT ?
            "#,
        )
        .bind(&q.exchange)
        .bind(&market_type)
        .bind(since_ms)
        .bind(top_n)
        .fetch_all::<TopMarketRow>()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("top market query failed for trend signals: {e:?}");
            return Json(vec![]);
        }
    };

    let mut out = Vec::with_capacity(top_markets.len());

    for m in top_markets {
        match fetch_hourly_klines(&client, m.market_id, days).await {
            Ok(bars) => {
                if let Some(point) = build_signal_point(&m.symbol, &bars) {
                    out.push(point);
                }
            }
            Err(e) => {
                warn!(
                    "trend signals skipped symbol={} due to error: {e:?}",
                    m.symbol
                );
            }
        }
    }

    Json(out)
}

async fn resolve_series_for_analytics(
    client: &Client,
    q: &AnalyticsQuery,
    default_top_n: usize,
    default_days: u32,
) -> Vec<SymbolSeries> {
    let days = parse_period_days(q.period.as_deref(), default_days);
    let tf_h = timeframe_hours(q.timeframe.as_deref());
    match build_universe_refs(client, q, default_top_n).await {
        Ok(refs) => load_symbol_series(client, &refs, days, tf_h).await,
        Err(e) => {
            error!("failed building analytics universe: {e:?}");
            Vec::new()
        }
    }
}

async fn get_zscore_overview(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, 30, 30).await;
    if series.is_empty() {
        return Json(json!({
            "zscoreTimeSeries": [],
            "statistics": {"mean": 0.0, "std": 0.0, "min": 0.0, "max": 0.0, "current": 0.0},
            "zscoreDistribution": {"buckets": [-3,-2,-1,0,1,2,3], "counts": [0,0,0,0,0,0,0]}
        }));
    }

    let tf_h = timeframe_hours(q.timeframe.as_deref());
    let daily_steps = (24 / tf_h.max(1)).max(1);
    let mut rows: Vec<Value> = Vec::new();
    let mut all_z = Vec::new();

    for s in series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let returns = pct_returns(&prices);
        let log_rets = log_returns(&prices);
        let n = prices.len();
        let win = (n / 6).clamp(12, 96);
        let z = zscore_series(&prices, win);

        let dollar_vol: Vec<f64> = s.bars.iter().map(|b| b.close_px * b.volume).collect();
        let roll_dollar = ema(&dollar_vol, win.max(12));
        let roll_dollar_z = zscore_series(&roll_dollar, win.max(12));
        let (mn, mx) = minmax(&roll_dollar);
        let denom = (mx - mn).max(1e-9);

        for i in 0..n {
            let ret_1d = if i >= daily_steps && prices[i - daily_steps] > 0.0 {
                (prices[i] / prices[i - daily_steps]) - 1.0
            } else {
                0.0
            };
            let zi = finite(z[i]);
            all_z.push(zi);
            rows.push(json!({
                "timestamp": s.bars[i].time_ms / 1000,
                "price": finite(prices[i]),
                "zscore": zi,
                "volume": finite(dollar_vol[i]),
                "returns1h": finite(*returns.get(i).unwrap_or(&0.0)),
                "returns1d": finite(ret_1d),
                "logReturns1h": finite(*log_rets.get(i).unwrap_or(&0.0)),
                "rollingDollarVolume": finite(*roll_dollar.get(i).unwrap_or(&0.0)),
                "rollingDollarVolumeZ": finite(*roll_dollar_z.get(i).unwrap_or(&0.0)),
                "rollingDollarVolumeNorm": finite((roll_dollar[i] - mn) / denom),
                "symbol": s.symbol.clone()
            }));
        }
    }

    let mu = mean(&all_z);
    let sd = variance(&all_z).sqrt();
    let mn = all_z.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = all_z.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let current = *all_z.last().unwrap_or(&0.0);
    let (buckets, counts) = histogram(&all_z, 7, -3.0, 3.0);

    Json(json!({
        "zscoreTimeSeries": rows,
        "statistics": {
            "mean": finite(mu),
            "std": finite(sd),
            "min": if mn.is_finite() { mn } else { 0.0 },
            "max": if mx.is_finite() { mx } else { 0.0 },
            "current": finite(current),
        },
        "zscoreDistribution": {
            "buckets": buckets,
            "counts": counts
        },
        "currentZScore": finite(current)
    }))
}

async fn get_volatility_analysis(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, 50, 30).await;
    if series.is_empty() {
        return Json(json!({
            "volatilityTimeSeries": [],
            "volVsReturns": [],
            "rangeDistribution": {"buckets": [], "counts": []}
        }));
    }

    let mut vol_rows = Vec::new();
    let mut vol_ret_rows = Vec::new();
    let mut ranges = Vec::new();

    for s in series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let vol = rolling_std(&rets, 24);
        let vol_z = zscore_series(&vol, 48);

        for i in 0..s.bars.len() {
            let bar = &s.bars[i];
            let range = if bar.close_px > 0.0 {
                (bar.high_px - bar.low_px) / bar.close_px
            } else {
                0.0
            };
            ranges.push(range);
            let point = json!({
                "timestamp": bar.time_ms / 1000,
                "volatility": finite(*vol.get(i).unwrap_or(&0.0)),
                "volatilityZScore": finite(*vol_z.get(i).unwrap_or(&0.0)),
                "returns": finite(*rets.get(i).unwrap_or(&0.0)),
                "high": finite(bar.high_px),
                "low": finite(bar.low_px),
                "range": finite(range),
                "symbol": s.symbol.clone(),
                "volume": finite(bar.close_px * bar.volume)
            });
            vol_rows.push(point.clone());
            vol_ret_rows.push(json!({
                "volZScore": finite(*vol_z.get(i).unwrap_or(&0.0)),
                "returns": finite(*rets.get(i).unwrap_or(&0.0)),
                "volume": finite(bar.close_px * bar.volume)
            }));
        }
    }

    let (buckets, counts) = histogram(&ranges, 12, 0.0, percentile(&ranges, 0.99).max(0.01));

    Json(json!({
        "volatilityTimeSeries": vol_rows,
        "volVsReturns": vol_ret_rows,
        "rangeDistribution": {
            "buckets": buckets,
            "counts": counts
        }
    }))
}

async fn get_cross_asset_analytics(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let index = q
        .index
        .clone()
        .or(q.index_coin.clone())
        .unwrap_or_else(|| "BTC".to_string())
        .to_ascii_uppercase();

    let series = resolve_series_for_analytics(&client, &q, 20, 30).await;
    if series.is_empty() {
        return Json(json!({
            "index": index,
            "correlationMatrix": {"coins": [], "matrix": [], "timestamp": Utc::now().timestamp_millis()},
            "coinBetas": {"coins": [], "betas": []},
            "rollingCorr": [],
            "rollingBeta": []
        }));
    }

    let mut coins = Vec::new();
    let mut returns_map: HashMap<String, Vec<f64>> = HashMap::new();
    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        if rets.len() > 8 {
            coins.push(s.symbol.clone());
            returns_map.insert(s.symbol.clone(), rets);
        }
    }
    if coins.is_empty() {
        return Json(json!({
            "index": index,
            "correlationMatrix": {"coins": [], "matrix": [], "timestamp": Utc::now().timestamp_millis()},
            "coinBetas": {"coins": [], "betas": []},
            "rollingCorr": [],
            "rollingBeta": []
        }));
    }

    if !coins.iter().any(|c| c == &index) {
        coins.insert(0, index.clone());
        returns_map.insert(index.clone(), vec![0.0; 64]);
    }

    let n = coins.len();
    let mut corr_m = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let a = returns_map
                .get(&coins[i])
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let b = returns_map
                .get(&coins[j])
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            corr_m[i][j] = if i == j { 1.0 } else { correlation(a, b) };
        }
    }

    let idx_rets = returns_map.get(&index).cloned().unwrap_or_default();
    let betas: Vec<f64> = coins
        .iter()
        .map(|c| {
            returns_map
                .get(c)
                .map(|r| if c == &index { 1.0 } else { beta(r, &idx_rets) })
                .unwrap_or(0.0)
        })
        .collect();

    let roll_win = q.window.unwrap_or(24).clamp(6, 240);
    let mut rolling_corr: Vec<Vec<f64>> = Vec::new();
    let mut rolling_beta: Vec<Vec<f64>> = Vec::new();
    for c in &coins {
        let r = returns_map.get(c).cloned().unwrap_or_default();
        let m = r.len().min(idx_rets.len());
        if m < roll_win + 1 {
            rolling_corr.push(vec![]);
            rolling_beta.push(vec![]);
            continue;
        }
        let ra = &r[r.len() - m..];
        let rb = &idx_rets[idx_rets.len() - m..];
        let mut cvec = Vec::new();
        let mut bvec = Vec::new();
        for i in (roll_win - 1)..m {
            let a = &ra[i + 1 - roll_win..=i];
            let b = &rb[i + 1 - roll_win..=i];
            cvec.push(if c == &index { 1.0 } else { correlation(a, b) });
            bvec.push(if c == &index { 1.0 } else { beta(a, b) });
        }
        rolling_corr.push(cvec);
        rolling_beta.push(bvec);
    }

    Json(json!({
        "index": index,
        "correlationMatrix": {
            "coins": coins,
            "matrix": corr_m,
            "timestamp": Utc::now().timestamp_millis()
        },
        "coinBetas": {
            "coins": coins,
            "betas": betas
        },
        "rollingCorr": rolling_corr,
        "rollingBeta": rolling_beta
    }))
}

async fn get_inter_asset_zscore(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, 20, 30).await;
    if series.len() < 2 {
        return Json(json!({
            "zscoreCorrelationMatrix": {"coins": [], "matrix": []},
            "zscoreBetaMatrix": {"coins": [], "matrix": []},
            "pairDivergence": []
        }));
    }

    let mut coins = Vec::new();
    let mut z_map: HashMap<String, Vec<f64>> = HashMap::new();
    let mut ts_map: HashMap<String, Vec<i64>> = HashMap::new();

    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let z = zscore_series(&rets, 24);
        coins.push(s.symbol.clone());
        z_map.insert(s.symbol.clone(), z);
        ts_map.insert(
            s.symbol.clone(),
            s.bars.iter().map(|b| b.time_ms / 1000).collect(),
        );
    }

    let n = coins.len();
    let mut corr_m = vec![vec![0.0; n]; n];
    let mut beta_m = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let a = z_map.get(&coins[i]).map(|v| v.as_slice()).unwrap_or(&[]);
            let b = z_map.get(&coins[j]).map(|v| v.as_slice()).unwrap_or(&[]);
            corr_m[i][j] = if i == j { 1.0 } else { correlation(a, b) };
            beta_m[i][j] = if i == j { 1.0 } else { beta(a, b) };
        }
    }

    let mut pair_div = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let z1 = z_map.get(&coins[i]).cloned().unwrap_or_default();
            let z2 = z_map.get(&coins[j]).cloned().unwrap_or_default();
            let ts = ts_map.get(&coins[i]).cloned().unwrap_or_default();
            let m = z1.len().min(z2.len()).min(ts.len());
            if m < 4 {
                continue;
            }
            let mut series_rows = Vec::new();
            let start = m.saturating_sub(120);
            for k in start..m {
                series_rows.push(json!({
                    "timestamp": ts[k],
                    "zscore1": finite(z1[k]),
                    "zscore2": finite(z2[k]),
                    "divergence": finite((z1[k] - z2[k]).abs())
                }));
            }
            pair_div.push(json!({
                "pair": format!("{}-{}", coins[i], coins[j]),
                "timeSeries": series_rows
            }));
        }
    }

    Json(json!({
        "zscoreCorrelationMatrix": {"coins": coins, "matrix": corr_m},
        "zscoreBetaMatrix": {"coins": coins, "matrix": beta_m},
        "pairDivergence": pair_div
    }))
}

async fn get_vol_liquidity(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, 10, 30).await;
    if series.is_empty() {
        return Json(json!({
            "volZScoreTimeSeries": [],
            "volZScoreVsReturns": [],
            "volumeDistribution": {"buckets": [], "counts": [], "currentValue": 0.0, "currentZScore": 0.0},
            "volumeSummaries": [],
            "volumeSummarySeries": [],
            "spreadSummarySeries": []
        }));
    }

    let target_coin = q
        .coin
        .clone()
        .unwrap_or_else(|| series[0].symbol.clone())
        .to_ascii_uppercase();
    let main = series
        .iter()
        .find(|s| s.symbol == target_coin)
        .cloned()
        .unwrap_or_else(|| series[0].clone());

    let prices: Vec<f64> = main.bars.iter().map(|b| b.close_px).collect();
    let returns = pct_returns(&prices);
    let vol = rolling_std(&returns, q.vol_window.unwrap_or(24).clamp(6, 240));
    let vol_z = zscore_series(&vol, 48);
    let dollar_vol: Vec<f64> = main.bars.iter().map(|b| b.close_px * b.volume).collect();
    let dollar_vol_z = zscore_series(&dollar_vol, 48);
    let ewma_dollar = ema(&dollar_vol, 60);
    let weekly_sum = rolling_sum(&dollar_vol, 24 * 7);
    let avg_daily = mean(&dollar_vol) * 24.0;
    let avg_hourly = mean(&dollar_vol);
    let (hist_b, hist_c) = histogram(&dollar_vol_z, 13, -3.0, 3.0);

    let mut vol_series = Vec::new();
    let mut vol_vs_ret = Vec::new();
    let mut volume_summary_series = Vec::new();
    let mut spread_summary_series = Vec::new();

    for i in 0..main.bars.len() {
        let bar = &main.bars[i];
        let spread1h = if bar.close_px > 0.0 {
            (bar.high_px - bar.low_px) / bar.close_px
        } else {
            0.0
        };
        vol_series.push(json!({
            "timestamp": bar.time_ms / 1000,
            "volZScore": finite(*vol_z.get(i).unwrap_or(&0.0)),
            "threshold2Sigma": 2.0,
            "thresholdNeg2Sigma": -2.0,
            "symbol": main.symbol.clone()
        }));
        vol_vs_ret.push(json!({
            "symbol": main.symbol.clone(),
            "volZScore": finite(*vol_z.get(i).unwrap_or(&0.0)),
            "dailyRange": finite(spread1h),
            "volume": finite(*dollar_vol.get(i).unwrap_or(&0.0)),
            "returns": finite(*returns.get(i).unwrap_or(&0.0))
        }));
        volume_summary_series.push(json!({
            "timestamp": bar.time_ms / 1000,
            "weeklyDollarVolume": finite(*weekly_sum.get(i).unwrap_or(&0.0)),
            "avgDailyDollarVolume": finite(avg_daily),
            "avgHourlyDollarVolume": finite(avg_hourly),
            "ewmaDollarVolume": finite(*ewma_dollar.get(i).unwrap_or(&0.0)),
            "dollarVolume": finite(*dollar_vol.get(i).unwrap_or(&0.0))
        }));

        let start_1d = i.saturating_sub(23);
        let spreads_1d: Vec<f64> = main.bars[start_1d..=i]
            .iter()
            .map(|b| {
                if b.close_px > 0.0 {
                    (b.high_px - b.low_px) / b.close_px
                } else {
                    0.0
                }
            })
            .collect();
        let start_7d = i.saturating_sub(24 * 7 - 1);
        let spreads_7d: Vec<f64> = main.bars[start_7d..=i]
            .iter()
            .map(|b| {
                if b.close_px > 0.0 {
                    (b.high_px - b.low_px) / b.close_px
                } else {
                    0.0
                }
            })
            .collect();
        let m1 = mean(&spreads_1d);
        let s1 = variance(&spreads_1d).sqrt();
        let m7 = mean(&spreads_7d);
        let s7 = variance(&spreads_7d).sqrt();
        let z = if s1 > 1e-12 {
            (spread1h - m1) / s1
        } else {
            0.0
        };
        spread_summary_series.push(json!({
            "timestamp": bar.time_ms / 1000,
            "spread1h": finite(spread1h),
            "avg1d": finite(m1),
            "std1d": finite(s1),
            "avg7d": finite(m7),
            "std7d": finite(s7),
            "avgZScore": finite(z)
        }));
    }

    let mut summaries = Vec::new();
    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let dv: Vec<f64> = s
            .bars
            .iter()
            .zip(prices.iter())
            .map(|(b, p)| b.volume * p)
            .collect();
        let weekly = rolling_sum(&dv, 24 * 7);
        let ewma = ema(&dv, 60);
        let current = *dv.last().unwrap_or(&0.0);
        let avg_d = mean(&dv) * 24.0;
        let avg_h = mean(&dv);
        let w = *weekly.last().unwrap_or(&0.0);
        let e = *ewma.last().unwrap_or(&0.0);
        summaries.push(json!({
            "symbol": s.symbol,
            "weeklyDollarVolume": finite(w),
            "avgDailyDollarVolume": finite(avg_d),
            "avgHourlyDollarVolume": finite(avg_h),
            "currentDollarVolume": finite(current),
            "ratioCurrentToAvgDaily": if avg_d > 1e-12 { finite(current / avg_d) } else { 0.0 },
            "ratioEWMAToAvgDaily": if avg_d > 1e-12 { finite(e / avg_d) } else { 0.0 }
        }));
    }

    Json(json!({
        "volZScoreTimeSeries": vol_series,
        "volZScoreVsReturns": vol_vs_ret,
        "volumeDistribution": {
            "buckets": hist_b,
            "counts": hist_c,
            "currentValue": finite(*dollar_vol_z.last().unwrap_or(&0.0)),
            "currentZScore": finite(*dollar_vol_z.last().unwrap_or(&0.0))
        },
        "volumeSummaries": summaries,
        "volumeSummarySeries": [{
            "symbol": main.symbol.clone(),
            "series": volume_summary_series
        }],
        "spreadSummarySeries": [{
            "symbol": main.symbol.clone(),
            "series": spread_summary_series
        }]
    }))
}

async fn get_leaders_laggards(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, 30, 30).await;
    if series.is_empty() {
        return Json(json!({
            "leaders": [],
            "laggards": [],
            "volumeSpikes": [],
            "decorrelated": [],
            "leadLagMatrix": {"coins": [], "lags": [], "matrix": []}
        }));
    }

    let mut rank_rows = Vec::new();
    let mut volume_spikes = Vec::new();
    let mut ret_map: HashMap<String, Vec<f64>> = HashMap::new();

    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let z = zscore_series(&rets, 48);
        let latest_z = *z.last().unwrap_or(&0.0);
        let latest_ret = *rets.last().unwrap_or(&0.0);
        let latest_vol = s.bars.last().map(|b| b.close_px * b.volume).unwrap_or(0.0);
        let vol_series: Vec<f64> = s.bars.iter().map(|b| b.close_px * b.volume).collect();
        let vol_z = zscore_series(&vol_series, 48);
        let latest_vol_z = *vol_z.last().unwrap_or(&0.0);
        rank_rows.push((s.symbol.clone(), latest_z, latest_ret, latest_vol));
        volume_spikes.push(json!({
            "symbol": s.symbol.clone(),
            "volumeZScore": finite(latest_vol_z),
            "priceChange": finite(latest_ret)
        }));
        ret_map.insert(s.symbol.clone(), rets);
    }

    rank_rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    let leaders: Vec<Value> = rank_rows
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, (sym, z, r, v))| {
            json!({"symbol": sym, "zscore": finite(*z), "returns": finite(*r), "volume": finite(*v), "rank": i + 1})
        })
        .collect();
    let laggards: Vec<Value> = rank_rows
        .iter()
        .rev()
        .take(10)
        .enumerate()
        .map(|(i, (sym, z, r, v))| {
            json!({"symbol": sym, "zscore": finite(*z), "returns": finite(*r), "volume": finite(*v), "rank": i + 1})
        })
        .collect();

    let coins: Vec<String> = series.iter().take(10).map(|s| s.symbol.clone()).collect();
    let lags = vec![-3, -2, -1, 0, 1, 2, 3];
    let mut lead_lag_matrix = Vec::new();
    for lag in &lags {
        let mut mat = vec![vec![0.0; coins.len()]; coins.len()];
        for i in 0..coins.len() {
            for j in 0..coins.len() {
                let a = ret_map.get(&coins[i]).cloned().unwrap_or_default();
                let b = ret_map.get(&coins[j]).cloned().unwrap_or_default();
                mat[i][j] = if i == j {
                    1.0
                } else {
                    lagged_correlation(&a, &b, *lag)
                };
            }
        }
        lead_lag_matrix.push(mat);
    }

    let mut decorrelated = Vec::new();
    for c in &coins {
        let mut corr_sum = 0.0;
        let mut count = 0usize;
        for d in &coins {
            if c == d {
                continue;
            }
            let a = ret_map.get(c).cloned().unwrap_or_default();
            let b = ret_map.get(d).cloned().unwrap_or_default();
            corr_sum += correlation(&a, &b).abs();
            count += 1;
        }
        let avg_corr = if count > 0 {
            corr_sum / count as f64
        } else {
            0.0
        };
        decorrelated.push(json!({
            "symbol": c,
            "correlationWithMarket": finite(1.0 - avg_corr),
            "avgCorrelation": finite(avg_corr)
        }));
    }
    decorrelated.sort_by(|a, b| {
        let av = a
            .get("correlationWithMarket")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let bv = b
            .get("correlationWithMarket")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        bv.total_cmp(&av)
    });
    volume_spikes.sort_by(|a, b| {
        let av = a
            .get("volumeZScore")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .abs();
        let bv = b
            .get("volumeZScore")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .abs();
        bv.total_cmp(&av)
    });

    Json(json!({
        "leaders": leaders,
        "laggards": laggards,
        "volumeSpikes": volume_spikes.into_iter().take(10).collect::<Vec<_>>(),
        "decorrelated": decorrelated.into_iter().take(10).collect::<Vec<_>>(),
        "leadLagMatrix": {
            "coins": coins,
            "lags": lags,
            "matrix": lead_lag_matrix
        }
    }))
}

async fn get_microstructure_flow(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let days = parse_period_days(q.period.as_deref(), 7);
    let refs = match build_universe_refs(&client, &q, q.top_n.unwrap_or(30)).await {
        Ok(v) => v,
        Err(e) => {
            error!("microstructure universe failed: {e:?}");
            Vec::new()
        }
    };
    if refs.is_empty() {
        return Json(json!({
            "volumeFlows": [],
            "rotationMatrix": {"coins": [], "flows": []},
            "liquidityConcentration": {"groups": []}
        }));
    }

    let since_ms = Utc::now().timestamp_millis() - (days as i64) * 24 * 60 * 60 * 1000;
    let mut rows = Vec::new();
    let mut totals = Vec::new();
    for r in &refs {
        match fetch_trade_rows(&client, r.market_id, since_ms).await {
            Ok(trades) => {
                let mut buy = 0.0;
                let mut sell = 0.0;
                for t in trades {
                    if t.side.eq_ignore_ascii_case("buy") {
                        buy += t.quote_qty.abs();
                    } else {
                        sell += t.quote_qty.abs();
                    }
                }
                let total = buy + sell;
                let net = buy - sell;
                rows.push((r.symbol.clone(), buy, sell, net, total));
                totals.push(net);
            }
            Err(e) => warn!("microstructure trades failed for {}: {e:?}", r.symbol),
        }
    }
    if rows.is_empty() {
        return Json(json!({
            "volumeFlows": [],
            "rotationMatrix": {"coins": [], "flows": []},
            "liquidityConcentration": {"groups": []}
        }));
    }

    let mu = mean(&totals);
    let sd = variance(&totals).sqrt().max(1e-9);
    let volume_flows: Vec<Value> = rows
        .iter()
        .map(|(s, b, o, n, _)| {
            json!({
                "symbol": s,
                "volumeIn": finite(*b),
                "volumeOut": finite(*o),
                "netFlow": finite(*n),
                "netFlowZScore": finite((*n - mu) / sd)
            })
        })
        .collect();

    let coins: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
    let mut flows = vec![vec![0.0; coins.len()]; coins.len()];
    for i in 0..coins.len() {
        for j in 0..coins.len() {
            if i == j {
                continue;
            }
            let ni = rows[i].3;
            let nj = rows[j].3;
            flows[i][j] = finite((ni - nj).max(0.0));
        }
    }

    let mut by_total = rows
        .iter()
        .map(|(s, _, _, _, t)| (s.clone(), *t))
        .collect::<Vec<_>>();
    by_total.sort_by(|a, b| b.1.total_cmp(&a.1));
    let total_notional: f64 = by_total.iter().map(|(_, v)| *v).sum::<f64>().max(1e-9);
    let n = by_total.len().max(1);
    let top5 = by_total.iter().take(5).map(|(_, v)| *v).sum::<f64>();
    let mid = by_total
        .iter()
        .skip(5)
        .take(5.min(n.saturating_sub(5)))
        .map(|(_, v)| *v)
        .sum::<f64>();
    let rest = (total_notional - top5 - mid).max(0.0);
    let groups = vec![
        json!({
            "range": "Top 5",
            "volumeShare": finite(top5 / total_notional),
            "countShare": finite((5.min(n) as f64) / n as f64)
        }),
        json!({
            "range": "6-10",
            "volumeShare": finite(mid / total_notional),
            "countShare": finite((5.min(n.saturating_sub(5)) as f64) / n as f64)
        }),
        json!({
            "range": "Rest",
            "volumeShare": finite(rest / total_notional),
            "countShare": finite((n.saturating_sub(10) as f64) / n as f64)
        }),
    ];

    Json(json!({
        "volumeFlows": volume_flows,
        "rotationMatrix": {
            "coins": coins,
            "flows": flows
        },
        "liquidityConcentration": {
            "groups": groups
        }
    }))
}

async fn get_relative_strength_overview(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let base = q
        .base
        .clone()
        .or(q.base_coin.clone())
        .unwrap_or_else(|| "BTC".to_string())
        .to_ascii_uppercase();

    let mut series = resolve_series_for_analytics(&client, &q, q.top_n.unwrap_or(20), 30).await;
    if !series.iter().any(|s| s.symbol == base) {
        let market_type = normalize_market_type(&q.market_type);
        if let Ok(Some(market_id)) =
            resolve_market_id(&client, &q.exchange, &market_type, &base).await
        {
            let days = parse_period_days(q.period.as_deref(), 30);
            let timeframe_h = timeframe_hours(q.timeframe.as_deref());
            if let Ok(bars) = fetch_hourly_klines(&client, market_id, days).await {
                let bars = resample_bars(&bars, timeframe_h);
                if bars.len() >= 8 {
                    series.push(SymbolSeries {
                        symbol: base.clone(),
                        market_id,
                        bars,
                    });
                }
            }
        }
    }

    if series.is_empty() {
        return Json(json!({
            "base": base,
            "momentumFactorLoadings": [],
            "pairDivergence": [],
            "persistence": [],
            "rsRankings": [],
            "rsSeries": []
        }));
    }

    let base_series = series
        .iter()
        .find(|s| s.symbol == base)
        .cloned()
        .unwrap_or_else(|| series[0].clone());
    let base_prices: Vec<f64> = base_series.bars.iter().map(|b| b.close_px).collect();
    let base_rets = pct_returns(&base_prices);
    let base_z = zscore_series(&base_rets, 48);

    let mut factor = Vec::new();
    let mut pair_div = Vec::new();
    let mut persistence = Vec::new();
    let mut rankings = Vec::new();
    let mut rs_series = Vec::new();

    for s in &series {
        if s.symbol == base_series.symbol {
            continue;
        }
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let z = zscore_series(&rets, 48);
        let m = rets.len().min(base_rets.len());
        if m < 8 {
            continue;
        }
        let r = &rets[rets.len() - m..];
        let b = &base_rets[base_rets.len() - m..];
        let beta_v = beta(r, b);
        let r2_v = r2(r, b);
        factor.push(json!({"symbol": s.symbol, "beta": finite(beta_v), "r2": finite(r2_v)}));
        persistence.push(json!({"symbol": s.symbol, "rho1": finite(autocorr_lag1(r))}));

        let mut rel_path = Vec::new();
        let mut v = 1.0;
        let ts_len = m.min(s.bars.len());
        for i in 0..ts_len {
            let sr = r[i];
            let br = b[i];
            v *= (1.0 + sr) / (1.0 + br);
            rel_path.push(json!({
                "timestamp": s.bars[s.bars.len() - ts_len + i].time_ms / 1000,
                "value": finite(v - 1.0)
            }));
        }
        rs_series.push(json!({"symbol": s.symbol, "series": rel_path.clone()}));

        let rs_vals: Vec<f64> = rel_path
            .iter()
            .filter_map(|x| x.get("value").and_then(|v| v.as_f64()))
            .collect();
        let rs_z = zscore_series(&rs_vals, 48);
        rankings.push(json!({
            "symbol": s.symbol,
            "z": finite(*rs_z.last().unwrap_or(&0.0))
        }));

        let bm = base_z.len().min(z.len()).min(ts_len);
        let mut div_ts = Vec::new();
        for i in 0..bm.min(120) {
            let idx = bm - bm.min(120) + i;
            let ts = s.bars[s.bars.len() - bm + idx].time_ms / 1000;
            let z1 = base_z[base_z.len() - bm + idx];
            let z2 = z[z.len() - bm + idx];
            div_ts.push(json!({
                "timestamp": ts,
                "z1": finite(z1),
                "z2": finite(z2),
                "divergence": finite(z2 - z1),
                "zscore": finite((z2 - z1).abs()),
                "spread": finite(z2 - z1)
            }));
        }
        pair_div.push(json!({
            "pair": format!("{}-{}", base, s.symbol),
            "timeSeries": div_ts
        }));
    }

    rankings.sort_by(|a, b| {
        let av = a.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bv = b.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
        bv.total_cmp(&av)
    });
    let rankings = rankings
        .into_iter()
        .enumerate()
        .map(|(i, mut row)| {
            if let Some(obj) = row.as_object_mut() {
                obj.insert("rank".to_string(), json!(i + 1));
            }
            row
        })
        .collect::<Vec<_>>();

    Json(json!({
        "base": base,
        "momentumFactorLoadings": factor,
        "pairDivergence": pair_div,
        "persistence": persistence,
        "rsRankings": rankings,
        "rsSeries": rs_series
    }))
}

async fn get_regime_momentum(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, q.top_n.unwrap_or(12), 90).await;
    if series.is_empty() {
        return Json(json!({
            "heatmap": {"coins": [], "timeframes": [], "matrix": []},
            "transitionMatrix": [[0.0,0.0,0.0],[0.0,0.0,0.0],[0.0,0.0,0.0]],
            "velocitySeries": [],
            "crossAssetDivergence": {"leaders": [], "laggards": [], "score": 0.0}
        }));
    }

    let mut heat_rows = Vec::new();
    let mut velocity_series = Vec::new();
    let mut momentum_rank = Vec::new();

    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let n = prices.len();
        let r1h = *rets.last().unwrap_or(&0.0);
        let r4h = if n > 4 && prices[n - 5] > 0.0 {
            (prices[n - 1] / prices[n - 5]) - 1.0
        } else {
            0.0
        };
        let r1d = if n > 24 && prices[n - 25] > 0.0 {
            (prices[n - 1] / prices[n - 25]) - 1.0
        } else {
            0.0
        };
        heat_rows.push(vec![finite(r1h), finite(r4h), finite(r1d)]);
        momentum_rank.push((s.symbol.clone(), (r1h + r4h + r1d) / 3.0));

        let mut points = Vec::new();
        let start = rets.len().saturating_sub(120);
        for i in start..rets.len() {
            let vel = rets[i];
            let acc = if i > 0 { vel - rets[i - 1] } else { 0.0 };
            let ts = s.bars[i].time_ms / 1000;
            points.push(
                json!({"timestamp": ts, "velocity": finite(vel), "acceleration": finite(acc)}),
            );
        }
        velocity_series.push(json!({"symbol": s.symbol, "series": points}));
    }

    momentum_rank.sort_by(|a, b| b.1.total_cmp(&a.1));
    let leaders: Vec<String> = momentum_rank
        .iter()
        .take(5)
        .map(|(s, _)| s.clone())
        .collect();
    let laggards: Vec<String> = momentum_rank
        .iter()
        .rev()
        .take(5)
        .map(|(s, _)| s.clone())
        .collect();
    let score = mean(&momentum_rank.iter().map(|(_, v)| *v).collect::<Vec<_>>());

    let bull = momentum_rank.iter().filter(|(_, v)| *v > 0.01).count() as f64;
    let bear = momentum_rank.iter().filter(|(_, v)| *v < -0.01).count() as f64;
    let range = momentum_rank.len() as f64 - bull - bear;
    let total = (bull + bear + range).max(1.0);
    let transition = vec![
        vec![bull / total, bear / total * 0.4, range / total * 0.6],
        vec![bull / total * 0.3, bear / total, range / total * 0.7],
        vec![bull / total * 0.5, bear / total * 0.5, range / total],
    ];

    Json(json!({
        "heatmap": {
            "coins": series.iter().map(|s| s.symbol.clone()).collect::<Vec<_>>(),
            "timeframes": ["1h", "4h", "1d"],
            "matrix": heat_rows
        },
        "transitionMatrix": transition,
        "velocitySeries": velocity_series,
        "crossAssetDivergence": {
            "leaders": leaders,
            "laggards": laggards,
            "score": finite(score)
        }
    }))
}

async fn get_market_seasonality(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, q.top_n.unwrap_or(10), 90).await;
    if series.is_empty() {
        return Json(json!({
            "intradayHeatmap": [],
            "weekdayHeatmap": {"volatility": [], "volume": [], "returns": []},
            "monthlySeasonality": {},
            "volumeAutocorrelation": {},
            "anomalies": [],
            "volumePersistence": [],
            "mostActivePeriods": [],
            "leastActivePeriods": [],
            "strongestPatterns": [],
            "volatilityClusters": [],
            "weekendEffect": [],
            "timezoneEffects": []
        }));
    }

    let mut intraday = Vec::new();
    let mut weekday_vol = Vec::new();
    let mut weekday_volume = Vec::new();
    let mut weekday_ret = Vec::new();
    let mut monthly = serde_json::Map::new();
    let mut vol_autocorr = serde_json::Map::new();
    let mut anomalies = Vec::new();
    let mut persistence = Vec::new();
    let mut weekend_effect = Vec::new();

    let mut global_hour_vol = vec![0.0_f64; 24];
    let mut global_hour_cnt = vec![0usize; 24];

    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let vol = rolling_std(&rets, 24);

        let mut h_ret_sum = [0.0_f64; 24];
        let mut h_ret_cnt = [0usize; 24];
        let mut wd_ret_sum = [0.0_f64; 7];
        let mut wd_ret_cnt = [0usize; 7];
        let mut wd_vol_sum = [0.0_f64; 7];
        let mut wd_vol_cnt = [0usize; 7];
        let mut wd_volume_sum = [0.0_f64; 7];
        let mut m_ret_sum = [0.0_f64; 12];
        let mut m_ret_cnt = [0usize; 12];
        let mut m_vol_sum = [0.0_f64; 12];
        let mut m_vol_cnt = [0usize; 12];
        let mut weekend_vals = Vec::new();
        let mut weekday_vals = Vec::new();

        for i in 0..s.bars.len() {
            let ts = s.bars[i].time_ms;
            let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts) else {
                continue;
            };
            let hour = dt.hour() as usize;
            let weekday = dt.weekday().num_days_from_monday() as usize;
            let month = (dt.month0()) as usize;
            let r = *rets.get(i).unwrap_or(&0.0);
            let v = *vol.get(i).unwrap_or(&0.0);
            let dv = s.bars[i].close_px * s.bars[i].volume;

            h_ret_sum[hour] += r;
            h_ret_cnt[hour] += 1;
            wd_ret_sum[weekday] += r;
            wd_ret_cnt[weekday] += 1;
            wd_vol_sum[weekday] += v;
            wd_vol_cnt[weekday] += 1;
            wd_volume_sum[weekday] += dv;

            m_ret_sum[month] += r;
            m_ret_cnt[month] += 1;
            m_vol_sum[month] += v;
            m_vol_cnt[month] += 1;

            global_hour_vol[hour] += dv;
            global_hour_cnt[hour] += 1;

            if weekday >= 5 {
                weekend_vals.push(r);
            } else {
                weekday_vals.push(r);
            }
        }

        let h_row: Vec<f64> = (0..24)
            .map(|h| {
                if h_ret_cnt[h] > 0 {
                    h_ret_sum[h] / h_ret_cnt[h] as f64
                } else {
                    0.0
                }
            })
            .collect();
        intraday.push(h_row);

        let wd_ret_row: Vec<f64> = (0..7)
            .map(|d| {
                if wd_ret_cnt[d] > 0 {
                    wd_ret_sum[d] / wd_ret_cnt[d] as f64
                } else {
                    0.0
                }
            })
            .collect();
        let wd_vol_row: Vec<f64> = (0..7)
            .map(|d| {
                if wd_vol_cnt[d] > 0 {
                    wd_vol_sum[d] / wd_vol_cnt[d] as f64
                } else {
                    0.0
                }
            })
            .collect();
        let wd_volume_row: Vec<f64> = (0..7)
            .map(|d| {
                if wd_ret_cnt[d] > 0 {
                    wd_volume_sum[d] / wd_ret_cnt[d] as f64
                } else {
                    0.0
                }
            })
            .collect();
        weekday_ret.push(wd_ret_row);
        weekday_vol.push(wd_vol_row);
        weekday_volume.push(wd_volume_row);

        let m_ret_row: Vec<f64> = (0..12)
            .map(|m| {
                if m_ret_cnt[m] > 0 {
                    m_ret_sum[m] / m_ret_cnt[m] as f64
                } else {
                    0.0
                }
            })
            .collect();
        let m_vol_row: Vec<f64> = (0..12)
            .map(|m| {
                if m_vol_cnt[m] > 0 {
                    m_vol_sum[m] / m_vol_cnt[m] as f64
                } else {
                    0.0
                }
            })
            .collect();
        monthly.insert(
            s.symbol.clone(),
            json!({
                "returns": m_ret_row,
                "volatility": m_vol_row
            }),
        );

        let dv_series: Vec<f64> = s.bars.iter().map(|b| b.close_px * b.volume).collect();
        vol_autocorr.insert(
            s.symbol.clone(),
            json!([
                finite(autocorr_lag1(&dv_series)),
                finite(lagged_correlation(&dv_series, &dv_series, 4)),
                finite(lagged_correlation(&dv_series, &dv_series, 24))
            ]),
        );

        let latest_hour = s
            .bars
            .last()
            .map(|b| {
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(b.time_ms)
                    .map(|dt| dt.hour() as usize)
            })
            .flatten();
        if let Some(h) = latest_hour {
            let hour_vals: Vec<f64> = s
                .bars
                .iter()
                .zip(rets.iter())
                .filter_map(|(b, r)| {
                    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(b.time_ms)
                        .filter(|dt| dt.hour() as usize == h)
                        .map(|_| *r)
                })
                .collect();
            let hm = mean(&hour_vals);
            let hs = variance(&hour_vals).sqrt();
            let lr = *rets.last().unwrap_or(&0.0);
            let z = if hs > 1e-12 { (lr - hm) / hs } else { 0.0 };
            if z.abs() >= 1.5 {
                anomalies.push(json!({
                    "symbol": s.symbol,
                    "period": format!("{h}:00"),
                    "metric": "returns",
                    "current": finite(lr),
                    "historical": finite(hm),
                    "zscore": finite(z)
                }));
            }
        }

        persistence.push(json!({
            "symbol": s.symbol,
            "lag1h": finite(autocorr_lag1(&dv_series)),
            "lag4h": finite(lagged_correlation(&dv_series, &dv_series, 4)),
            "lag24h": finite(lagged_correlation(&dv_series, &dv_series, 24)),
            "pattern": if autocorr_lag1(&dv_series) > 0.5 { "persistent" } else { "mean-reverting" }
        }));

        weekend_effect.push(json!({
            "asset": s.symbol,
            "effect": finite(mean(&weekend_vals) - mean(&weekday_vals))
        }));
    }

    let mut hour_rank: Vec<(usize, f64)> = (0..24)
        .map(|h| {
            let avg = if global_hour_cnt[h] > 0 {
                global_hour_vol[h] / global_hour_cnt[h] as f64
            } else {
                0.0
            };
            (h, avg)
        })
        .collect();
    hour_rank.sort_by(|a, b| b.1.total_cmp(&a.1));

    let most_active = hour_rank
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, (h, v))| {
            json!({"id": format!("top-{i}"), "asset": "All", "period": format!("{h}:00"), "volatility": 0.0, "volume": finite(*v)})
        })
        .collect::<Vec<_>>();
    let least_active = hour_rank
        .iter()
        .rev()
        .take(5)
        .enumerate()
        .map(|(i, (h, v))| {
            json!({"id": format!("low-{i}"), "asset": "All", "period": format!("{h}:00"), "volatility": 0.0, "volume": finite(*v)})
        })
        .collect::<Vec<_>>();

    let strongest_patterns = vec![
        json!({"id": "p1", "name": "Intraday return seasonality", "strength": finite(mean(&hour_rank.iter().map(|(_, v)| *v).collect::<Vec<_>>()))}),
        json!({"id": "p2", "name": "Weekday volume clustering", "strength": finite(percentile(&hour_rank.iter().map(|(_, v)| *v).collect::<Vec<_>>(), 0.9))}),
    ];

    let mut clusters = Vec::new();
    let mut highs = Vec::new();
    let mut mids = Vec::new();
    let mut lows = Vec::new();
    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let v = variance(&rets).sqrt();
        if v > 0.04 {
            highs.push(s.symbol.clone());
        } else if v > 0.02 {
            mids.push(s.symbol.clone());
        } else {
            lows.push(s.symbol.clone());
        }
    }
    clusters.push(json!({"id": "high-vol", "period": "all", "assets": highs}));
    clusters.push(json!({"id": "mid-vol", "period": "all", "assets": mids}));
    clusters.push(json!({"id": "low-vol", "period": "all", "assets": lows}));

    let tz = vec![
        json!({"zone": "Asia", "activeHours": "00:00-08:00 UTC", "impact": "high"}),
        json!({"zone": "Europe", "activeHours": "08:00-16:00 UTC", "impact": "medium"}),
        json!({"zone": "US", "activeHours": "16:00-24:00 UTC", "impact": "high"}),
    ];

    Json(json!({
        "intradayHeatmap": intraday,
        "weekdayHeatmap": {
            "volatility": weekday_vol,
            "volume": weekday_volume,
            "returns": weekday_ret
        },
        "monthlySeasonality": monthly,
        "volumeAutocorrelation": vol_autocorr,
        "anomalies": anomalies,
        "volumePersistence": persistence,
        "mostActivePeriods": most_active,
        "leastActivePeriods": least_active,
        "strongestPatterns": strongest_patterns,
        "volatilityClusters": clusters,
        "weekendEffect": weekend_effect,
        "timezoneEffects": tz
    }))
}

async fn get_volatility_dynamics(
    State(client): State<Client>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<Value> {
    let series = resolve_series_for_analytics(&client, &q, q.top_n.unwrap_or(20), 90).await;
    if series.is_empty() {
        return Json(json!({
            "vovTimeSeries": [],
            "skewnessTimeSeries": [],
            "skewKurtosisScatter": [],
            "volDistribution": {"bins": [], "frequencies": []},
            "distributionStats": [],
            "covarianceTimeSeries": [],
            "regimeClassification": [],
            "instabilityRankings": [],
            "flowVolBeta": [],
            "volStatsSummary": []
        }));
    }

    let vol_window = q.vol_window.unwrap_or(24).clamp(6, 240);
    let vov_window = q.vov_window.unwrap_or(96).clamp(12, 400);
    let moment_window = q.moment_window.unwrap_or(48).clamp(12, 240);
    let histogram_coin = q
        .histogram_coin
        .clone()
        .unwrap_or_else(|| series[0].symbol.clone())
        .to_ascii_uppercase();

    let mut benchmark_returns = vec![0.0; series.iter().map(|s| s.bars.len()).max().unwrap_or(0)];
    let mut bcnt = vec![0usize; benchmark_returns.len()];
    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let r = pct_returns(&prices);
        for (i, v) in r.iter().enumerate() {
            if i < benchmark_returns.len() {
                benchmark_returns[i] += *v;
                bcnt[i] += 1;
            }
        }
    }
    for i in 0..benchmark_returns.len() {
        if bcnt[i] > 0 {
            benchmark_returns[i] /= bcnt[i] as f64;
        }
    }

    let mut vov_series = Vec::new();
    let mut skew_series = Vec::new();
    let mut cov_series = Vec::new();
    let mut scatter = Vec::new();
    let mut regimes = Vec::new();
    let mut instability = Vec::new();
    let mut flow_beta = Vec::new();
    let mut vol_stats = Vec::new();
    let mut hist_source = Vec::new();

    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let returns = pct_returns(&prices);
        let vol = rolling_std(&returns, vol_window);
        let vov = rolling_std(&vol, vov_window);
        let vol_z = zscore_series(&vol, 48);

        let m = returns.len().min(benchmark_returns.len());
        let r_tail = &returns[returns.len().saturating_sub(m)..];
        let b_tail = &benchmark_returns[benchmark_returns.len().saturating_sub(m)..];

        let mut skew_pts = Vec::new();
        let mut vov_pts = Vec::new();
        let mut cov_pts = Vec::new();
        for i in 0..s.bars.len() {
            let ts = s.bars[i].time_ms / 1000;
            vov_pts.push(json!({"timestamp": ts, "value": finite(*vov.get(i).unwrap_or(&0.0))}));

            let start = i.saturating_sub(moment_window - 1);
            let sl = &returns[start..=i];
            let sk = skewness(sl);
            skew_pts.push(json!({"timestamp": ts, "value": finite(sk)}));

            if i < r_tail.len() && i < b_tail.len() {
                let cstart = i.saturating_sub(vol_window - 1);
                let a = &r_tail[cstart..=i];
                let b = &b_tail[cstart..=i];
                cov_pts.push(json!({"timestamp": ts, "value": finite(covariance(a, b))}));
            }
        }

        let latest_sk = skew_pts
            .last()
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let latest_kurt = kurtosis(&returns[returns.len().saturating_sub(moment_window)..]);
        let latest_vov = *vov.last().unwrap_or(&0.0);
        let latest_vol = *vol.last().unwrap_or(&0.0);
        let avg_vol = mean(&vol);
        let perc = if vol.is_empty() {
            0.0
        } else {
            let less = vol.iter().filter(|x| **x <= latest_vol).count() as f64;
            less / vol.len() as f64
        };

        let regime = if latest_vov > percentile(&vov, 0.9) {
            "unstable"
        } else if latest_sk > 0.75 {
            "euphoric"
        } else if latest_sk < -0.75 {
            "stressed"
        } else if latest_vol < percentile(&vol, 0.2) {
            "compressed"
        } else {
            "normal"
        };

        let status = if latest_vov > percentile(&vov, 0.9) {
            "critical"
        } else if latest_vov > percentile(&vov, 0.75) {
            "warning"
        } else {
            "normal"
        };

        let dv: Vec<f64> = s.bars.iter().map(|b| b.close_px * b.volume).collect();
        let dv_chg = pct_returns(&dv);
        let vol_chg = pct_returns(&vol);
        let flow_b = beta(&dv_chg, &vol_chg);
        let flow_r2 = r2(&dv_chg, &vol_chg);

        vov_series.push(json!({"symbol": s.symbol, "data": vov_pts}));
        skew_series.push(json!({"symbol": s.symbol, "data": skew_pts}));
        cov_series.push(json!({"symbol": s.symbol, "data": cov_pts}));
        scatter.push(json!({"symbol": s.symbol, "skewness": finite(latest_sk), "kurtosis": finite(latest_kurt)}));
        regimes.push(json!({
            "symbol": s.symbol,
            "regime": regime,
            "volatility": finite(latest_vol),
            "skewness": finite(latest_sk),
            "kurtosis": finite(latest_kurt)
        }));
        instability.push(json!({
            "symbol": s.symbol,
            "vov": finite(latest_vov),
            "skewness": finite(latest_sk),
            "status": status,
            "status_label": status.to_ascii_uppercase()
        }));
        flow_beta.push(json!({
            "symbol": s.symbol,
            "beta": finite(flow_b),
            "r_squared": finite(flow_r2)
        }));
        vol_stats.push(json!({
            "symbol": s.symbol,
            "current_vol": finite(latest_vol),
            "avg_vol": finite(avg_vol),
            "percentile": finite(perc),
            "skewness": finite(latest_sk),
            "kurtosis": finite(latest_kurt),
            "vov": finite(latest_vov)
        }));

        if s.symbol == histogram_coin {
            hist_source = vol.clone();
        }

        let _ = vol_z;
    }

    if hist_source.is_empty() {
        hist_source = vol_stats
            .iter()
            .filter_map(|v| v.get("current_vol").and_then(|x| x.as_f64()))
            .collect();
    }
    let bin_step = q.bin_step_pct.unwrap_or(5.0).clamp(1.0, 20.0) / 100.0;
    let max_v = percentile(&hist_source, 0.995).max(bin_step);
    let bins_n = ((max_v / bin_step).ceil() as usize).clamp(6, 80);
    let (bins, freq) = histogram(&hist_source, bins_n, 0.0, max_v.max(bin_step));

    let ds = vec![
        json!({"label": "Mean", "value": finite(mean(&hist_source)), "line": true, "color": "#60a5fa"}),
        json!({"label": "Std", "value": finite(variance(&hist_source).sqrt()), "line": false}),
        json!({"label": "P95", "value": finite(percentile(&hist_source, 0.95)), "line": true, "color": "#f59e0b"}),
    ];

    Json(json!({
        "vovTimeSeries": vov_series,
        "skewnessTimeSeries": skew_series,
        "skewKurtosisScatter": scatter,
        "volDistribution": {
            "bins": bins,
            "frequencies": freq
        },
        "distributionStats": ds,
        "covarianceTimeSeries": cov_series,
        "regimeClassification": regimes,
        "instabilityRankings": instability,
        "flowVolBeta": flow_beta,
        "volStatsSummary": vol_stats
    }))
}

async fn get_trades_analysis(
    State(client): State<Client>,
    Query(q): Query<TradesAnalysisQuery>,
) -> Json<Value> {
    let assets = {
        let parsed = parse_csv_symbols(q.assets.as_deref());
        if parsed.is_empty() {
            vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()]
        } else {
            parsed
        }
    };
    let interval = q
        .interval
        .unwrap_or_else(|| "5m".to_string())
        .to_ascii_lowercase();
    let interval_ms: i64 = match interval.as_str() {
        "1m" => 60_000,
        "5m" => 300_000,
        "15m" => 900_000,
        "1h" => 3_600_000,
        "4h" => 14_400_000,
        _ => 300_000,
    };
    let period_hours = parse_period_hours(q.period.as_deref(), 24);
    let since_ms = Utc::now().timestamp_millis() - period_hours as i64 * 60 * 60 * 1000;

    let mut per_asset_trades: HashMap<String, Vec<TradeRow>> = HashMap::new();
    let mut all_trade_sizes = Vec::new();

    for sym in &assets {
        let market_id = match resolve_market_id(&client, "binance", "spot", sym).await {
            Ok(Some(v)) => v,
            _ => continue,
        };
        match fetch_trade_rows(&client, market_id, since_ms).await {
            Ok(trades) => {
                all_trade_sizes.extend(trades.iter().map(|t| t.quote_qty.abs()));
                per_asset_trades.insert(sym.clone(), trades);
            }
            Err(e) => warn!("trades analysis fetch failed for {sym}: {e:?}"),
        }
    }

    if per_asset_trades.is_empty() {
        return Json(json!({
            "imbalanceScatter": [],
            "sizeDistribution": {"buckets": [], "counts": []},
            "notionalRankings": [],
            "imbalanceTimeSeries": [],
            "velocityHeatmap": [],
            "whaleTrades": [],
            "correlationMatrix": [],
            "tradeZScores": [],
            "microstructureStats": []
        }));
    }

    let mut imbalance_scatter = Vec::new();
    let mut notional_rank = Vec::new();
    let mut imbalance_series = Vec::new();
    let mut whale = Vec::new();
    let mut z_count = Vec::new();
    let mut z_size = Vec::new();
    let mut z_buy = Vec::new();
    let mut asset_bucketed: HashMap<String, Vec<f64>> = HashMap::new();

    for (sym, trades) in &per_asset_trades {
        let mut buy = 0.0;
        let mut sell = 0.0;
        let mut sum_size = 0.0;
        let mut bucket: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
        for t in trades {
            let qv = t.quote_qty.abs();
            sum_size += qv;
            if t.side.eq_ignore_ascii_case("buy") {
                buy += qv;
                let b = t.trade_time_ms - (t.trade_time_ms % interval_ms);
                let e = bucket.entry(b).or_insert((0.0, 0.0));
                e.0 += qv;
            } else {
                sell += qv;
                let b = t.trade_time_ms - (t.trade_time_ms % interval_ms);
                let e = bucket.entry(b).or_insert((0.0, 0.0));
                e.1 += qv;
            }
            whale.push(json!({
                "id": format!("{}-{}", sym, t.trade_id),
                "symbol": sym,
                "timestamp": t.trade_time_ms,
                "size": qv,
                "price": t.price,
                "side": if t.side.eq_ignore_ascii_case("buy") { "buy" } else { "sell" }
            }));
        }
        let total = buy + sell;
        let buy_ratio = if total > 0.0 { buy / total } else { 0.0 };
        let avg_size = if trades.is_empty() {
            0.0
        } else {
            sum_size / trades.len() as f64
        };
        let imbalance = if total > 0.0 {
            (buy - sell) / total
        } else {
            0.0
        };
        imbalance_scatter.push(json!({
            "symbol": sym,
            "buyRatio": finite(buy_ratio * 100.0),
            "volume": finite(total),
            "avgTradeSize": finite(avg_size),
            "imbalance": finite(imbalance)
        }));
        notional_rank.push(json!({"symbol": sym, "avgSize": finite(avg_size)}));
        z_count.push(trades.len() as f64);
        z_size.push(avg_size);
        z_buy.push(buy_ratio * 100.0);

        let mut points = Vec::new();
        let mut seq = Vec::new();
        for (ts, (b, s)) in &bucket {
            let tot = *b + *s;
            let iv = if tot > 0.0 { (*b - *s) / tot } else { 0.0 };
            seq.push(iv);
            points.push(json!({"timestamp": *ts, "value": finite(iv * 100.0)}));
        }
        asset_bucketed.insert(sym.clone(), seq);
        imbalance_series.push(json!({"symbol": sym, "data": points}));
    }

    notional_rank.sort_by(|a, b| {
        let av = a.get("avgSize").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bv = b.get("avgSize").and_then(|v| v.as_f64()).unwrap_or(0.0);
        bv.total_cmp(&av)
    });
    whale.sort_by(|a, b| {
        let av = a.get("size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bv = b.get("size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        bv.total_cmp(&av)
    });

    let dist_asset = q
        .distribution_asset
        .unwrap_or_else(|| assets[0].clone())
        .to_ascii_uppercase();
    let dist_vals: Vec<f64> = per_asset_trades
        .get(&dist_asset)
        .map(|ts| ts.iter().map(|t| t.quote_qty.abs()).collect())
        .unwrap_or_else(|| all_trade_sizes.clone());
    let max_dist = percentile(&dist_vals, 0.995).max(1.0);
    let (dist_b, dist_c) = histogram(&dist_vals, 20, 0.0, max_dist);

    let mut corr_m = vec![vec![0.0; assets.len()]; assets.len()];
    for i in 0..assets.len() {
        for j in 0..assets.len() {
            let a = asset_bucketed.get(&assets[i]).cloned().unwrap_or_default();
            let b = asset_bucketed.get(&assets[j]).cloned().unwrap_or_default();
            corr_m[i][j] = if i == j { 1.0 } else { correlation(&a, &b) };
        }
    }

    let slot_hours = [0_u32, 4, 8, 12, 16, 20];
    let mut vel_heat = vec![vec![0.0; slot_hours.len()]; assets.len()];
    for (i, sym) in assets.iter().enumerate() {
        let trades = per_asset_trades.get(sym).cloned().unwrap_or_default();
        let mut slot_counts = vec![0usize; slot_hours.len()];
        let mut slot_minutes = vec![0.0_f64; slot_hours.len()];
        for t in trades {
            if let Some(dt) =
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(t.trade_time_ms)
            {
                let h = dt.hour();
                let slot = ((h / 4) as usize).min(5);
                slot_counts[slot] += 1;
                slot_minutes[slot] += 1.0;
            }
        }
        for s in 0..slot_hours.len() {
            vel_heat[i][s] = if slot_minutes[s] > 0.0 {
                slot_counts[s] as f64 / slot_minutes[s]
            } else {
                0.0
            };
        }
    }

    let count_mu = mean(&z_count);
    let count_sd = variance(&z_count).sqrt().max(1e-9);
    let size_mu = mean(&z_size);
    let size_sd = variance(&z_size).sqrt().max(1e-9);
    let buy_mu = mean(&z_buy);
    let buy_sd = variance(&z_buy).sqrt().max(1e-9);
    let trade_z = vec![
        json!({"name":"Trade Count", "type":"count", "current": finite(count_mu), "average": finite(count_mu), "zscore": finite((count_mu - count_mu)/count_sd)}),
        json!({"name":"Average Size", "type":"size", "current": finite(size_mu), "average": finite(size_mu), "zscore": finite((size_mu - size_mu)/size_sd)}),
        json!({"name":"Buy Ratio", "type":"percent", "current": finite(buy_mu), "average": finite(buy_mu), "zscore": finite((buy_mu - buy_mu)/buy_sd)}),
    ];

    let total_trades: usize = per_asset_trades.values().map(|v| v.len()).sum();
    let total_notional: f64 = per_asset_trades
        .values()
        .flat_map(|v| v.iter())
        .map(|t| t.quote_qty.abs())
        .sum();
    let micro_stats = vec![
        json!({"name": "Assets", "value": format!("{}", per_asset_trades.len())}),
        json!({"name": "Trades", "value": format!("{}", total_trades)}),
        json!({"name": "Notional", "value": format!("{:.2}", total_notional)}),
    ];

    Json(json!({
        "imbalanceScatter": imbalance_scatter,
        "sizeDistribution": {"buckets": dist_b, "counts": dist_c},
        "notionalRankings": notional_rank,
        "imbalanceTimeSeries": imbalance_series,
        "velocityHeatmap": vel_heat,
        "whaleTrades": whale.into_iter().take(25).collect::<Vec<_>>(),
        "correlationMatrix": corr_m,
        "tradeZScores": trade_z,
        "microstructureStats": micro_stats
    }))
}

async fn get_trend_expected(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Value> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let top_n = q.top_n.unwrap_or(20).clamp(1, 100);
    let refs =
        match fetch_top_markets_by_volume(&client, &q.exchange, &market_type, days, top_n).await {
            Ok(v) => v,
            Err(e) => {
                error!("trend expected failed to load markets: {e:?}");
                vec![]
            }
        };
    let series = load_symbol_series(&client, &refs, days, 1).await;
    let mut out = Vec::new();
    for s in series {
        if let Some(sig) = build_signal_point(&s.symbol, &s.bars) {
            out.push(json!({
                "symbol": s.symbol,
                "expected_return": finite(sig.composite),
                "confidence": finite(sig.composite.abs().min(1.0)),
                "trend": finite(sig.trend),
                "momentum": finite(sig.momentum)
            }));
        }
    }
    Json(json!(out))
}

async fn get_trend_regressions_xsec(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Value> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let top_n = q.top_n.unwrap_or(20).clamp(1, 100);
    let refs = fetch_top_markets_by_volume(&client, &q.exchange, &market_type, days, top_n)
        .await
        .unwrap_or_default();
    let series = load_symbol_series(&client, &refs, days, 1).await;
    let mut signals = Vec::new();
    let mut next_returns = Vec::new();
    for s in series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        if rets.len() < 2 {
            continue;
        }
        if let Some(sig) = build_signal_point(&s.symbol, &s.bars[..s.bars.len() - 1]) {
            signals.push(sig.composite);
            next_returns.push(*rets.last().unwrap_or(&0.0));
        }
    }
    let beta_v = beta(&next_returns, &signals);
    let alpha = mean(&next_returns) - beta_v * mean(&signals);
    let r2_v = r2(&next_returns, &signals);
    Json(json!([{
        "model": "next_return ~ signal",
        "alpha": finite(alpha),
        "beta": finite(beta_v),
        "r2": finite(r2_v),
        "n": next_returns.len()
    }]))
}

async fn get_trend_regressions_ts(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Value> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let top_n = q.top_n.unwrap_or(10).clamp(1, 50);
    let refs = fetch_top_markets_by_volume(&client, &q.exchange, &market_type, days, top_n)
        .await
        .unwrap_or_default();
    let series = load_symbol_series(&client, &refs, days, 1).await;
    let mut out = Vec::new();
    for s in series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        if rets.len() < 12 {
            continue;
        }
        let y = &rets[1..];
        let x = &rets[..rets.len() - 1];
        out.push(json!({
            "symbol": s.symbol,
            "alpha": finite(mean(y) - beta(y, x) * mean(x)),
            "beta": finite(beta(y, x)),
            "r2": finite(r2(y, x)),
            "n": y.len()
        }));
    }
    Json(json!(out))
}

async fn get_trend_vol_forecast(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Value> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let top_n = q.top_n.unwrap_or(20).clamp(1, 100);
    let refs = fetch_top_markets_by_volume(&client, &q.exchange, &market_type, days, top_n)
        .await
        .unwrap_or_default();
    let series = load_symbol_series(&client, &refs, days, 1).await;
    let mut out = Vec::new();
    for s in series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        let rets = pct_returns(&prices);
        let vol = rolling_std(&rets, 24);
        let fcst = ema(&vol, 24);
        out.push(json!({
            "symbol": s.symbol,
            "current_vol": finite(*vol.last().unwrap_or(&0.0)),
            "forecast_vol": finite(*fcst.last().unwrap_or(&0.0)),
            "series": s.bars.iter().zip(fcst.iter()).map(|(b, v)| json!({"timestamp": b.time_ms / 1000, "value": finite(*v)})).collect::<Vec<_>>()
        }));
    }
    Json(json!(out))
}

async fn get_trend_portfolio(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Value> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let top_n = q.top_n.unwrap_or(20).clamp(1, 100);
    let refs = fetch_top_markets_by_volume(&client, &q.exchange, &market_type, days, top_n)
        .await
        .unwrap_or_default();
    let series = load_symbol_series(&client, &refs, days, 1).await;

    let mut raw = Vec::new();
    for s in &series {
        if let Some(sig) = build_signal_point(&s.symbol, &s.bars) {
            let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
            let rets = pct_returns(&prices);
            let vol = rolling_std(&rets, 24);
            raw.push((s.symbol.clone(), sig.composite, *vol.last().unwrap_or(&0.0)));
        }
    }
    let gross_raw = raw.iter().map(|(_, x, _)| x.abs()).sum::<f64>().max(1e-9);
    let mut positions = Vec::new();
    for (sym, sig, vol) in raw {
        let w = sig / gross_raw;
        positions.push(json!({
            "asset": sym,
            "weight": finite(w),
            "volatility": finite(vol),
            "signal": finite(sig),
            "expected_return": finite(sig),
            "contribution": finite(w * sig)
        }));
    }
    let gross = positions
        .iter()
        .map(|p| {
            p.get("weight")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .abs()
        })
        .sum::<f64>();
    let net = positions
        .iter()
        .map(|p| p.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .sum::<f64>();

    Json(json!({
        "positions": positions,
        "summary": {
            "gross": finite(gross),
            "net": finite(net),
            "turnover": finite(gross)
        }
    }))
}

async fn get_trend_portfolio_constraints(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Value> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let refs = fetch_top_markets_by_volume(
        &client,
        &q.exchange,
        &market_type,
        days,
        q.top_n.unwrap_or(20).clamp(1, 100),
    )
    .await
    .unwrap_or_default();
    Json(json!({
        "constraints": refs.into_iter().map(|r| json!({
            "symbol": r.symbol,
            "max_weight": 0.15,
            "min_weight": -0.15
        })).collect::<Vec<_>>()
    }))
}

async fn get_trend_portfolio_risk_matrix(
    State(client): State<Client>,
    Query(q): Query<TrendQuery>,
) -> Json<Value> {
    let market_type = normalize_market_type(&q.market_type);
    let days = days_or_default(q.days);
    let refs = fetch_top_markets_by_volume(
        &client,
        &q.exchange,
        &market_type,
        days,
        q.top_n.unwrap_or(12).clamp(2, 40),
    )
    .await
    .unwrap_or_default();
    let series = load_symbol_series(&client, &refs, days, 1).await;
    let symbols: Vec<String> = series.iter().map(|s| s.symbol.clone()).collect();
    let mut matrix = vec![vec![0.0; symbols.len()]; symbols.len()];
    let mut ret_map = HashMap::new();
    for s in &series {
        let prices: Vec<f64> = s.bars.iter().map(|b| b.close_px).collect();
        ret_map.insert(s.symbol.clone(), pct_returns(&prices));
    }
    for i in 0..symbols.len() {
        for j in 0..symbols.len() {
            let a = ret_map.get(&symbols[i]).cloned().unwrap_or_default();
            let b = ret_map.get(&symbols[j]).cloned().unwrap_or_default();
            matrix[i][j] = if i == j { 1.0 } else { correlation(&a, &b) };
        }
    }
    Json(json!({
        "symbols": symbols,
        "matrix": matrix
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let client = migrations::create_pool().await?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/api/funding/matrix", get(get_funding_matrix))
        .route("/api/health", get(health))
        .route("/api/meta-data/options", get(get_meta_options))
        .route("/api/trend/signals/xsec", get(get_trend_signals_xsec))
        .route("/api/trend/series/price", get(get_trend_series_price))
        .route("/api/trend/series/returns", get(get_trend_series_returns))
        .route("/api/trend/series/vol", get(get_trend_series_vol))
        .route("/api/trend/series/volume", get(get_trend_series_volume))
        .route("/api/trend/expected", get(get_trend_expected))
        .route(
            "/api/trend/regressions/xsec",
            get(get_trend_regressions_xsec),
        )
        .route("/api/trend/regressions/ts", get(get_trend_regressions_ts))
        .route("/api/trend/vol/forecast", get(get_trend_vol_forecast))
        .route("/api/trend/portfolio", get(get_trend_portfolio))
        .route(
            "/api/trend/portfolio/constraints",
            get(get_trend_portfolio_constraints),
        )
        .route(
            "/api/trend/portfolio/risk_matrix",
            get(get_trend_portfolio_risk_matrix),
        )
        .route("/api/statistics/zscore/overview", get(get_zscore_overview))
        .route(
            "/api/statistics/volatility/analysis",
            get(get_volatility_analysis),
        )
        .route(
            "/api/statistics/volatility/liquidity",
            get(get_vol_liquidity),
        )
        .route(
            "/api/statistics/inter-asset/zscore",
            get(get_inter_asset_zscore),
        )
        .route(
            "/api/statistics/cross-asset/analytics",
            get(get_cross_asset_analytics),
        )
        .route(
            "/api/statistics/cross-section/leaders-laggards",
            get(get_leaders_laggards),
        )
        .route(
            "/api/statistics/microstructure/flow",
            get(get_microstructure_flow),
        )
        .route(
            "/api/statistics/relative-strength/overview",
            get(get_relative_strength_overview),
        )
        .route("/api/statistics/momentum/regime", get(get_regime_momentum))
        .route(
            "/api/zscore/market-seasonality",
            get(get_market_seasonality),
        )
        .route(
            "/api/zscore/volatility-dynamics",
            get(get_volatility_dynamics),
        )
        .route("/api/zscore/trades-analysis", get(get_trades_analysis))
        .route("/zscore/trades-analysis", get(get_trades_analysis))
        .with_state(client)
        .layer(cors);

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    info!("Server running at http://{}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
