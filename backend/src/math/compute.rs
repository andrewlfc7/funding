// src/math/compute.rs
use std::f64;

/// Numerically stable EMA (alpha in (0,1], seed = first value)
fn ema(values: &[f64], alpha: f64) -> Vec<f64> {
    assert!(alpha > 0.0 && alpha <= 1.0);
    if values.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(values.len());
    let mut s = values[0];
    out.push(s);
    for &x in &values[1..] {
        s = alpha * x + (1.0 - alpha) * s;
        out.push(s);
    }
    out
}

pub fn ewmac(
    prices: &[f64],
    lfast: usize,
    lslow: Option<usize>,
    vol_span: usize,
    usescalar: bool,
    capmin: f64,
    capmax: f64,
) -> Vec<Option<f64>> {
    if prices.len() < 2 || lfast == 0 {
        return vec![None; prices.len()];
    }
    let lslow = lslow.unwrap_or(4 * lfast);
    if lslow <= lfast {
        return vec![None; prices.len()];
    }

    // EMA alphas
    let alpha_f = 2.0 / (lfast as f64 + 1.0);
    let alpha_s = 2.0 / (lslow as f64 + 1.0);

    // EMAs of price
    let fast = ema(prices, alpha_f);
    let slow = ema(prices, alpha_s);

    // Raw signal
    let mut raw = Vec::with_capacity(prices.len());
    for i in 0..prices.len() {
        raw.push(fast[i] - slow[i]);
    }

    // EWMA variance of deltas
    let mut deltas = Vec::with_capacity(prices.len());
    deltas.push(0.0);
    for i in 1..prices.len() {
        deltas.push(prices[i] - prices[i - 1]);
    }
    let alpha_v = 2.0 / (vol_span as f64 + 1.0);
    // Start variance with first squared delta (like python)
    let mut var = Vec::with_capacity(prices.len());
    let mut s = deltas[0] * deltas[0];
    var.push(s);
    for i in 1..deltas.len() {
        s = alpha_v * deltas[i].powi(2) + (1.0 - alpha_v) * s;
        var.push(s);
    }
    let vol: Vec<f64> = var.into_iter().map(|v| v.sqrt() + 1e-8).collect();

    // Optional scalar
    let scalar = if usescalar {
        ((lfast as f64 * lslow as f64) / (2.0 * ((lslow - lfast) as f64))).sqrt()
    } else {
        1.0
    };

    // Build output with Nones where slow EMA/vol aren’t “mature” yet.
    // We’ll be conservative and start giving values only after lslow.
    let mut out = vec![None; prices.len()];
    for i in 0..prices.len() {
        if i + 1 >= lslow {
            let z = (raw[i] / vol[i]) * scalar;
            let clipped = z.clamp(capmin, capmax);
            out[i] = Some(clipped);
        }
    }
    out
}

/// Breakout: closeness to trailing `window`-day high, returning:
///   half - days_since_high
/// where half = (window-1)/2, so +half means new high today, -half means high at window-1 days ago.
pub fn breakout(close: &[f64], window: usize) -> Vec<Option<f64>> {
    if window == 0 || close.is_empty() {
        return vec![None; close.len()];
    }
    let half = (window as f64 - 1.0) / 2.0;
    let mut out = vec![None; close.len()];
    for i in 0..close.len() {
        if i + 1 >= window {
            let start = i + 1 - window;
            let slice = &close[start..=i];
            // find index of max in slice
            let mut max_idx = 0usize;
            let mut max_val = f64::NEG_INFINITY;
            for (j, &v) in slice.iter().enumerate() {
                if v > max_val {
                    max_val = v;
                    max_idx = j;
                }
            }
            let days_since_high = (window - 1) - max_idx;
            let score = half - days_since_high as f64;
            out[i] = Some(score);
        }
    }
    out
}

/// Exponential weights of length n with a given half-life (Python's exp_weights).
fn exp_weights(n: usize, half_life: usize) -> Vec<f64> {
    // w_t ∝ exp( -ln(2) * (n-1 - t) / half_life ), newest has largest weight
    if n == 0 {
        return vec![];
    }
    let hl = half_life.max(1) as f64;
    let lambda = (2.0f64).ln() / hl;
    let mut w = Vec::with_capacity(n);
    for t in 0..n {
        let age = (n - 1 - t) as f64;
        w.push((-lambda * age).exp());
    }
    // normalize
    let sum: f64 = w.iter().sum();
    if sum > 0.0 {
        for x in &mut w {
            *x /= sum;
        }
    }
    w
}

/// Momentum (weighted cumulative product of (1 + returns), lagged by `lag` days).
/// Returns Vec<Option<f64>> aligned to input length; None until `trailing_days`+lag-1.
pub fn momentum(
    returns: &[f64],
    trailing_days: usize,
    half_life: usize,
    lag: usize,
) -> Vec<Option<f64>> {
    if trailing_days == 0 || returns.is_empty() {
        return vec![None; returns.len()];
    }
    // apply lag
    let mut r = vec![0.0; returns.len()];
    for i in 0..returns.len() {
        if i >= lag {
            r[i] = returns[i - lag];
        } else {
            r[i] = 0.0;
        }
    }

    let w = exp_weights(trailing_days, half_life);
    let mut out = vec![None; returns.len()];

    // rolling window weighted cumprod - 1 of (1 + r_t * weight_t)
    for i in 0..r.len() {
        if i + 1 >= trailing_days + lag {
            let start = i + 1 - trailing_days;
            let slice = &r[start..=i];
            // align weights to slice length (they always match trailing_days here)
            let mut cum = 1.0;
            for (rv, &wt) in slice.iter().zip(w.iter()) {
                cum *= 1.0 + rv * wt;
            }
            out[i] = Some(cum - 1.0);
        }
    }
    out
}
