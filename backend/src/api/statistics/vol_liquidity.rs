use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::{
    Tf, fetch_multi_hourly_ohlcv, histogram_counts, log_returns, parse_period_days,
    resample_from_hourly, rolling_mean_std, top_markets_by_usd_volume_live, zscore_series,
};

fn default_market_type() -> String {
    "spot".to_string()
}
fn default_timeframe() -> String {
    "1h".to_string()
}
#[inline]
fn finite(x: f64) -> f64 {
    if x.is_finite() { x } else { 0.0 }
}

#[derive(Debug, Deserialize)]
pub struct VolLiquidityRequest {
    #[serde(default)]
    pub coin: Option<String>,
    pub period: String,
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub marketType: String,
    #[serde(default)]
    pub topN: Option<i64>,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
}

#[derive(Debug, Serialize)]
pub struct VolLiquidityResponse {
    pub volZScoreTimeSeries: Vec<VolZRow>,
    pub volumeDistribution: VolumeDistribution,
    pub volZScoreVsReturns: Vec<VolVsRet>,
    /// existing per-symbol aggregates (kept for compatibility)
    pub volumeSummaries: Vec<VolumeSummary>,
    /// volume time series
    pub volumeSummarySeries: Vec<VolumeSummarySeries>,
    /// NEW: spread time series (per symbol)
    pub spreadSummarySeries: Vec<SpreadSummarySeries>, // <--- NEW
}

#[derive(Debug, Serialize)]
pub struct VolZRow {
    pub timestamp: i64,
    pub volZScore: f64,
    pub threshold2Sigma: f64,
    pub thresholdNeg2Sigma: f64,
    pub symbol: String,
}

#[derive(Debug, Serialize)]
pub struct VolumeDistribution {
    pub buckets: Vec<f64>,
    pub counts: Vec<usize>,
    pub currentZScore: f64,
}

#[derive(Debug, Serialize)]
pub struct VolVsRet {
    pub volZScore: f64,
    pub dailyRange: f64,
    pub volume: f64, // USD notional of that bar
    pub symbol: String,
}

/// existing: weekly & average USD volumes + ratios
#[derive(Debug, Serialize)]
pub struct VolumeSummary {
    pub symbol: String,
    pub weeklyDollarVolume: f64,
    pub avgDailyDollarVolume: f64,
    pub avgHourlyDollarVolume: f64,
    pub currentDollarVolume: f64,
    pub ratioCurrentToAvgDaily: f64,
    pub ratioEWMAToAvgDaily: f64,
}

/// volume series point + per-symbol container
#[derive(Debug, Serialize)]
pub struct VolumeSummaryPoint {
    pub timestamp: i64,
    pub weeklyDollarVolume: f64,
    pub avgDailyDollarVolume: f64,
    pub avgHourlyDollarVolume: f64,
    pub ewmaDollarVolume: f64,
    pub dollarVolume: f64,
}

#[derive(Debug, Serialize)]
pub struct VolumeSummarySeries {
    pub symbol: String,
    pub series: Vec<VolumeSummaryPoint>,
}

/// NEW: spread series point + per-symbol container
#[derive(Debug, Serialize)]
pub struct SpreadSummaryPoint {
    pub timestamp: i64,
    pub spread1h: f64,  // (high-low)/mid
    pub avg1d: f64,     // rolling mean over 1 TF-day
    pub std1d: f64,     // rolling std over 1 TF-day
    pub avg7d: f64,     // rolling mean over 7 TF-days
    pub std7d: f64,     // rolling std over 7 TF-days
    pub avgZScore: f64, // (avg1d - avg7d)/std7d
}

#[derive(Debug, Serialize)]
pub struct SpreadSummarySeries {
    pub symbol: String,
    pub series: Vec<SpreadSummaryPoint>,
}

pub async fn get_vol_liquidity(
    State(pool): State<PgPool>,
    Query(q): Query<VolLiquidityRequest>,
) -> Json<VolLiquidityResponse> {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);
    let since_unix =
        (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

    // ===== Universe mode =====
    if let Some(n) = q.topN {
        let top = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, n)
            .await
            .unwrap_or_default();

        if top.is_empty() {
            return Json(VolLiquidityResponse {
                volZScoreTimeSeries: vec![],
                volumeDistribution: VolumeDistribution {
                    buckets: vec![],
                    counts: vec![],
                    currentZScore: 0.0,
                },
                volZScoreVsReturns: vec![],
                volumeSummaries: vec![],
                volumeSummarySeries: vec![],
                spreadSummarySeries: vec![], // NEW
            });
        }

        let mids: Vec<i32> = top.iter().map(|(_, mid, _)| *mid).collect();
        let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix)
            .await
            .unwrap_or_default();

        let mut rows: Vec<VolZRow> = Vec::new();
        let mut scatter: Vec<VolVsRet> = Vec::new();
        let mut vol_ewma_z_pool: Vec<f64> = Vec::new();
        let mut last_volz_sum = 0.0;
        let mut last_volz_cnt = 0usize;
        let mut summaries: Vec<VolumeSummary> = Vec::new();
        let mut summary_series_all: Vec<VolumeSummarySeries> = Vec::new();
        let mut spread_series_all: Vec<SpreadSummarySeries> = Vec::new(); // NEW

        for (sym, mid, _) in top {
            let Some(hourly) = by_mid.get(&mid) else {
                continue;
            };
            let ser = resample_from_hourly(hourly, tf.period_secs());
            let n = ser.len();
            if n < 30 {
                continue;
            }

            let ts: Vec<i64> = ser.iter().map(|r| r.ts).collect();
            let close: Vec<f64> = ser.iter().map(|r| r.close).collect();
            let high: Vec<f64> = ser.iter().map(|r| r.high).collect();
            let low: Vec<f64> = ser.iter().map(|r| r.low).collect();
            let base: Vec<f64> = ser.iter().map(|r| r.volume).collect();
            let usd: Vec<f64> = close
                .iter()
                .zip(base.iter())
                .map(|(p, &v)| finite(p * v))
                .collect();

            // --- VOL metrics (existing) ---
            let lr = log_returns(&close);
            let win_fast_vol = tf.steps_per_day().max(6);
            let (_, s) = rolling_mean_std(&lr, win_fast_vol);
            let win_slow_vol = (5 * win_fast_vol).max(6).min(n.max(6));
            let (m_slow, sd_slow) = rolling_mean_std(&s, win_slow_vol);
            let volz: Vec<f64> = s
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let sd = sd_slow[i];
                    if sd.is_finite() && sd > 0.0 && m_slow[i].is_finite() {
                        (v - m_slow[i]) / sd
                    } else {
                        f64::NAN
                    }
                })
                .collect();

            // EWMA USD volume + its z (for distribution only)
            let vol_ewma = super::ewma_span(&usd, win_fast_vol.max(24));
            let vol_ewma_z = zscore_series(&vol_ewma, win_slow_vol);

            let start_a = volz.iter().position(|v| v.is_finite()).unwrap_or(n);
            let start_b = vol_ewma_z.iter().position(|v| v.is_finite()).unwrap_or(n);
            let start = start_a.max(start_b);
            if start >= n {
                continue;
            }

            // rows & scatter
            for i in start..n {
                let vz = finite(volz[i]);
                rows.push(VolZRow {
                    timestamp: ts[i],
                    volZScore: vz,
                    threshold2Sigma: 2.0,
                    thresholdNeg2Sigma: -2.0,
                    symbol: sym.clone(),
                });

                let steps_day = tf.steps_per_day();
                let dr = if i >= steps_day {
                    let lo = low[i - steps_day + 1..=i]
                        .iter()
                        .fold(f64::INFINITY, |a, &b| a.min(b));
                    let hi = high[i - steps_day + 1..=i]
                        .iter()
                        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                    if lo > 0.0 { (hi - lo) / lo } else { 0.0 }
                } else {
                    0.0
                };

                scatter.push(VolVsRet {
                    volZScore: vz,
                    dailyRange: finite(dr),
                    volume: finite(usd[i]),
                    symbol: sym.clone(),
                });

                vol_ewma_z_pool.push(finite(vol_ewma_z[i]));
            }

            if let Some(&lz) = volz[start..].last() {
                if lz.is_finite() {
                    last_volz_sum += lz;
                    last_volz_cnt += 1;
                }
            }

            // ===== Aggregate volume summary (unchanged) =====
            let end = n - 1;
            let steps_day = tf.steps_per_day();
            let steps_week = steps_day * 7;
            let start_week = end.saturating_sub(steps_week - 1).max(start);
            let weekly_sum = usd[start_week..=end]
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .sum::<f64>();
            let avg_daily = weekly_sum / 7.0;
            let avg_hourly = weekly_sum / (7.0 * steps_day as f64);
            let current = finite(usd[end]);
            let ewma_last = finite(vol_ewma[end]);

            summaries.push(VolumeSummary {
                symbol: sym.clone(),
                weeklyDollarVolume: finite(weekly_sum),
                avgDailyDollarVolume: finite(avg_daily),
                avgHourlyDollarVolume: finite(avg_hourly),
                currentDollarVolume: current,
                ratioCurrentToAvgDaily: if avg_daily > 0.0 {
                    current / avg_daily
                } else {
                    0.0
                },
                ratioEWMAToAvgDaily: if avg_daily > 0.0 {
                    ewma_last / avg_daily
                } else {
                    0.0
                },
            });

            // ===== Volume time series (existing) =====
            let mut psum = vec![0.0f64; n + 1];
            for i in 0..n {
                psum[i + 1] = psum[i] + if usd[i].is_finite() { usd[i] } else { 0.0 };
            }
            let mut v_series = Vec::with_capacity(n - start);
            let start_mature = start.max(steps_week.saturating_sub(1)); // ensure full 7d window

            for i in start_mature..n {
                let week_sum = psum[i + 1] - psum[i + 1 - steps_week.min(i + 1)];
                let avg_d = week_sum / 7.0;
                let avg_h = week_sum / (7.0 * steps_day as f64);
                v_series.push(VolumeSummaryPoint {
                    timestamp: ts[i],
                    weeklyDollarVolume: finite(week_sum),
                    avgDailyDollarVolume: finite(avg_d),
                    avgHourlyDollarVolume: finite(avg_h),
                    ewmaDollarVolume: finite(vol_ewma[i]),
                    dollarVolume: finite(usd[i]),
                });
            }
            summary_series_all.push(VolumeSummarySeries {
                symbol: sym.clone(),
                series: v_series,
            });

            // ===== NEW: Spread time series =====
            // spread1h = (high-low)/mid with mid=(high+low)/2
            let spread1h: Vec<f64> = high
                .iter()
                .zip(low.iter())
                .map(|(&h, &l)| {
                    if h.is_finite() && l.is_finite() && h > 0.0 && l > 0.0 {
                        let mid = (h + l) * 0.5;
                        if mid > 0.0 { (h - l) / mid } else { 0.0 }
                    } else {
                        0.0
                    }
                })
                .collect();

            let win_fast = steps_day; // 1 TF-day
            let win_slow = steps_day * 7; // 7 TF-days

            let (mean_fast, std_fast) = rolling_mean_std(&spread1h, win_fast);
            let (mean_slow, std_slow) = rolling_mean_std(&spread1h, win_slow);

            let start_spread = start.max(win_slow.saturating_sub(1));
            let capacity = n.saturating_sub(start_spread);
            if capacity == 0 {
                continue;
            }

            let mut s_series = Vec::with_capacity(capacity);

            for i in start_spread..n {
                let m1 = mean_fast[i];
                let s1 = std_fast[i];
                let m7 = mean_slow[i];
                let s7 = std_slow[i];
                let z = if s7.is_finite() && s7 > 0.0 && m1.is_finite() && m7.is_finite() {
                    (m1 - m7) / s7
                } else {
                    f64::NAN
                };

                s_series.push(SpreadSummaryPoint {
                    timestamp: ts[i],
                    spread1h: finite(spread1h[i]),
                    avg1d: finite(m1),
                    std1d: finite(s1),
                    avg7d: finite(m7),
                    std7d: finite(s7),
                    avgZScore: finite(z),
                });
            }
            spread_series_all.push(SpreadSummarySeries {
                symbol: sym.clone(),
                series: s_series,
            });
        }

        let buckets = vec![-3.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0];
        let counts = histogram_counts(&vol_ewma_z_pool, &buckets);
        let current = if last_volz_cnt > 0 {
            last_volz_sum / (last_volz_cnt as f64)
        } else {
            0.0
        };

        return Json(VolLiquidityResponse {
            volZScoreTimeSeries: rows,
            volumeDistribution: VolumeDistribution {
                buckets,
                counts,
                currentZScore: finite(current),
            },
            volZScoreVsReturns: scatter,
            volumeSummaries: summaries,
            volumeSummarySeries: summary_series_all,
            spreadSummarySeries: spread_series_all, // NEW
        });
    }

    // ===== Single-coin mode =====
    let Some(sym) = q.coin.as_deref() else {
        return Json(VolLiquidityResponse {
            volZScoreTimeSeries: vec![],
            volumeDistribution: VolumeDistribution {
                buckets: vec![],
                counts: vec![],
                currentZScore: 0.0,
            },
            volZScoreVsReturns: vec![],
            volumeSummaries: vec![],
            volumeSummarySeries: vec![],
            spreadSummarySeries: vec![], // NEW
        });
    };

    let ohlcv = super::get_ohlcv_resampled(&pool, &q.exchange, sym, &q.marketType, tf, days)
        .await
        .unwrap_or_default();
    if ohlcv.is_empty() {
        return Json(VolLiquidityResponse {
            volZScoreTimeSeries: vec![],
            volumeDistribution: VolumeDistribution {
                buckets: vec![],
                counts: vec![],
                currentZScore: 0.0,
            },
            volZScoreVsReturns: vec![],
            volumeSummaries: vec![],
            volumeSummarySeries: vec![],
            spreadSummarySeries: vec![], // NEW
        });
    }

    let n = ohlcv.len();
    let ts: Vec<i64> = ohlcv.iter().map(|r| r.ts).collect();
    let close: Vec<f64> = ohlcv.iter().map(|r| r.close).collect();
    let high: Vec<f64> = ohlcv.iter().map(|r| r.high).collect();
    let low: Vec<f64> = ohlcv.iter().map(|r| r.low).collect();
    let base: Vec<f64> = ohlcv.iter().map(|r| r.volume).collect();
    let usd: Vec<f64> = close
        .iter()
        .zip(base.iter())
        .map(|(p, &v)| finite(p * v))
        .collect();

    // VOL (existing)
    let lr = log_returns(&close);
    let win_fast_vol = tf.steps_per_day().max(6);
    let (_, s) = rolling_mean_std(&lr, win_fast_vol);
    let win_slow_vol = (5 * win_fast_vol).max(6).min(n.max(6));
    let (m_slow, sd_slow) = rolling_mean_std(&s, win_slow_vol);
    let volz: Vec<f64> = s
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let sd = sd_slow[i];
            if sd.is_finite() && sd > 0.0 && m_slow[i].is_finite() {
                (v - m_slow[i]) / sd
            } else {
                f64::NAN
            }
        })
        .collect();

    let vol_ewma = super::ewma_span(&usd, win_fast_vol.max(24));
    let vol_ewma_z = zscore_series(&vol_ewma, win_slow_vol);

    let start_a = volz.iter().position(|v| v.is_finite()).unwrap_or(n);
    let start_b = vol_ewma_z.iter().position(|v| v.is_finite()).unwrap_or(n);
    let start = start_a.max(start_b);
    if start >= n {
        return Json(VolLiquidityResponse {
            volZScoreTimeSeries: vec![],
            volumeDistribution: VolumeDistribution {
                buckets: vec![],
                counts: vec![],
                currentZScore: 0.0,
            },
            volZScoreVsReturns: vec![],
            volumeSummaries: vec![],
            volumeSummarySeries: vec![],
            spreadSummarySeries: vec![], // NEW
        });
    }

    let mut rows = Vec::with_capacity(n - start);
    let mut scatter = Vec::with_capacity(n - start);
    let mut vol_ewma_z_pool: Vec<f64> = Vec::with_capacity(n - start);

    for i in start..n {
        let vz = finite(volz[i]);
        rows.push(VolZRow {
            timestamp: ts[i],
            volZScore: vz,
            threshold2Sigma: 2.0,
            thresholdNeg2Sigma: -2.0,
            symbol: sym.to_string(),
        });

        let steps_day = tf.steps_per_day();
        let dr = if i >= steps_day {
            let lo = low[i - steps_day + 1..=i]
                .iter()
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let hi = high[i - steps_day + 1..=i]
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            if lo > 0.0 { (hi - lo) / lo } else { 0.0 }
        } else {
            0.0
        };

        scatter.push(VolVsRet {
            volZScore: vz,
            dailyRange: finite(dr),
            volume: finite(usd[i]),
            symbol: sym.to_string(),
        });

        vol_ewma_z_pool.push(finite(vol_ewma_z[i]));
    }

    // Volume aggregates (existing)
    let end = n - 1;
    let steps_day = tf.steps_per_day();
    let steps_week = steps_day * 7;
    let start_week = end.saturating_sub(steps_week - 1).max(start);
    let weekly_sum = usd[start_week..=end]
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .sum::<f64>();
    let avg_daily = weekly_sum / 7.0;
    let avg_hourly = weekly_sum / (7.0 * steps_day as f64);
    let current = finite(usd[end]);
    let ewma_last = finite(vol_ewma[end]);

    let summaries = vec![VolumeSummary {
        symbol: sym.to_string(),
        weeklyDollarVolume: finite(weekly_sum),
        avgDailyDollarVolume: finite(avg_daily),
        avgHourlyDollarVolume: finite(avg_hourly),
        currentDollarVolume: current,
        ratioCurrentToAvgDaily: if avg_daily > 0.0 {
            current / avg_daily
        } else {
            0.0
        },
        ratioEWMAToAvgDaily: if avg_daily > 0.0 {
            ewma_last / avg_daily
        } else {
            0.0
        },
    }];

    // Volume series (existing)
    let mut psum = vec![0.0f64; n + 1];
    for i in 0..n {
        psum[i + 1] = psum[i] + if usd[i].is_finite() { usd[i] } else { 0.0 };
    }
    let start_mature = start.max(steps_week.saturating_sub(1));
    let mut v_series = Vec::with_capacity(n - start_mature);
    for i in start_mature..n {
        let week_sum = psum[i + 1] - psum[i + 1 - steps_week.min(i + 1)];
        let avg_d = week_sum / 7.0;
        let avg_h = week_sum / (7.0 * steps_day as f64);
        v_series.push(VolumeSummaryPoint {
            timestamp: ts[i],
            weeklyDollarVolume: finite(week_sum),
            avgDailyDollarVolume: finite(avg_d),
            avgHourlyDollarVolume: finite(avg_h),
            ewmaDollarVolume: finite(vol_ewma[i]),
            dollarVolume: finite(usd[i]),
        });
    }

    // NEW: Spread series
    let spread1h: Vec<f64> = high
        .iter()
        .zip(low.iter())
        .map(|(&h, &l)| {
            if h.is_finite() && l.is_finite() && h > 0.0 && l > 0.0 {
                let mid = (h + l) * 0.5;
                if mid > 0.0 { (h - l) / mid } else { 0.0 }
            } else {
                0.0
            }
        })
        .collect();

    let win_fast = steps_day;
    let win_slow = steps_day * 7;
    let (mean_fast, std_fast) = rolling_mean_std(&spread1h, win_fast);
    let (mean_slow, std_slow) = rolling_mean_std(&spread1h, win_slow);

    let start_spread = start.max(win_slow.saturating_sub(1));
    let mut s_series = Vec::with_capacity(n - start_spread);
    for i in start_spread..n {
        let m1 = mean_fast[i];
        let s1 = std_fast[i];
        let m7 = mean_slow[i];
        let s7 = std_slow[i];
        let z = if s7.is_finite() && s7 > 0.0 && m1.is_finite() && m7.is_finite() {
            (m1 - m7) / s7
        } else {
            f64::NAN
        };

        s_series.push(SpreadSummaryPoint {
            timestamp: ts[i],
            spread1h: finite(spread1h[i]),
            avg1d: finite(m1),
            std1d: finite(s1),
            avg7d: finite(m7),
            std7d: finite(s7),
            avgZScore: finite(z),
        });
    }

    let buckets = vec![-3.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0];
    let counts = histogram_counts(&vol_ewma_z_pool, &buckets);
    let current_z = volz
        .iter()
        .skip(start)
        .rev()
        .find(|v| v.is_finite())
        .copied()
        .unwrap_or(0.0);

    Json(VolLiquidityResponse {
        volZScoreTimeSeries: rows,
        volumeDistribution: VolumeDistribution {
            buckets,
            counts,
            currentZScore: finite(current_z),
        },
        volZScoreVsReturns: scatter,
        volumeSummaries: summaries,
        volumeSummarySeries: vec![VolumeSummarySeries {
            symbol: sym.to_string(),
            series: v_series,
        }],
        spreadSummarySeries: vec![SpreadSummarySeries {
            symbol: sym.to_string(),
            series: s_series,
        }], // NEW
    })
}
