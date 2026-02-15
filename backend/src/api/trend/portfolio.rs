use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::error;

// ========================
// Query / Payload types
// ========================

#[derive(Debug, Deserialize)]
pub struct PortfolioParams {
    pub exchange: String,
    pub market_type: String,     // "spot" | "perps"
    pub variant_id: Option<i32>, // optional; if present we load constraints

    // Sigmoid mapping (computed only; not persisted)
    pub center: Option<f64>, // default 0.0
    pub slope: Option<f64>,  // default 1.0
    pub cap: Option<f64>,    // default 1.0
    pub bias: Option<f64>,   // default 0.0

    // Optional symbol filter (comma-separated or repeated)
    #[serde(default, alias = "symbols")]
    pub symbol: Option<String>,

    // Optional: limit how stale signals can be (days)
    pub since_days: Option<i64>, // default 30
}

#[derive(Debug, Deserialize)]
pub struct UpsertConstraints {
    pub variant_id: Option<i32>, // if absent, we’ll create a variant
    pub name: Option<String>,    // for auto-create; required if variant_id is None

    pub gross_cap: Option<f64>,    // e.g., 1.0
    pub max_pos: Option<f64>,      // e.g., 0.10
    pub turnover_cap: Option<f64>, // optional
    pub vol_target: Option<f64>,   // optional daily target
    pub lambda_risk: Option<f64>,  // optional; placeholder for future
    pub gamma_cost: Option<f64>,   // optional; placeholder for future
    pub ignore_open: Option<bool>, // default false
}

// ========================
// Response types
// ========================

#[derive(Debug, Serialize)]
pub struct PortfolioResponse {
    pub exchange: String,
    pub market_type: String,
    pub variant_id: Option<i32>,
    pub params: PortfolioParamsOut,
    pub counts: PortfolioCounts,
    pub gross_before: f64,
    pub gross_after: f64,
    pub rows: Vec<PortfolioRow>,
}

#[derive(Debug, Serialize)]
pub struct PortfolioParamsOut {
    pub mapping: SigmoidParams,
    pub constraints: ConstraintsOut,
}

#[derive(Debug, Serialize)]
pub struct ConstraintsOut {
    pub gross_cap: f64,
    pub max_pos: f64,
    pub turnover_cap: Option<f64>,
    pub vol_target: Option<f64>,
    pub lambda_risk: Option<f64>,
    pub gamma_cost: Option<f64>,
    pub ignore_open: bool,
}

#[derive(Debug, Serialize)]
pub struct PortfolioCounts {
    pub symbols: usize,
    pub with_signal: usize,
}

#[derive(Debug, Serialize)]
pub struct PortfolioRow {
    pub symbol: String,
    pub ts: i64,              // millis
    pub signal: f64,          // latest composite
    pub exposure_raw: f64,    // sigmoid(signal)
    pub exposure_capped: f64, // after per-name cap
    pub exposure_final: f64,  // after gross scaling
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct SigmoidParams {
    pub center: f64,
    pub slope: f64,
    pub cap: f64,
    pub bias: f64,
}

// ========================
// Public router
// ========================

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/trend/portfolio", get(get_portfolio_preview))
        .route("/trend/portfolio/constraints", put(put_constraints))
}

// ========================
// GET /trend/portfolio
// ========================

pub async fn get_portfolio_preview(
    State(pool): State<PgPool>,
    Query(q): Query<PortfolioParams>,
) -> Result<Json<PortfolioResponse>, (StatusCode, String)> {
    let mt = parse_market_type(&q.market_type).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Load constraints (if variant given) or fall back to sane defaults
    let default_constraints = ConstraintsOut {
        gross_cap: 1.0,
        max_pos: 0.10,
        turnover_cap: None,
        vol_target: None,
        lambda_risk: None,
        gamma_cost: None,
        ignore_open: false,
    };
    let constraints = if let Some(vid) = q.variant_id {
        load_constraints(&pool, vid)
            .await
            .unwrap_or(default_constraints)
    } else {
        default_constraints
    };

    // Mapping params (in-memory only)
    let mapping = SigmoidParams {
        center: q.center.unwrap_or(0.0),
        slope: q.slope.unwrap_or(1.0),
        cap: q.cap.unwrap_or(1.0).abs().min(1.0),
        bias: q.bias.unwrap_or(0.0),
    };

    // Fetch latest composite signal per symbol
    let since_days = q.since_days.unwrap_or(30).max(1);
    let symbol_filter: Option<Vec<String>> = q.symbol.as_ref().map(|s| {
        s.split(',')
            .map(|x| x.trim().to_ascii_uppercase())
            .filter(|x| !x.is_empty())
            .collect()
    });

    let rows = if let Some(ref syms) = symbol_filter {
        sqlx::query(
            r#"
            SELECT DISTINCT ON (symbol)
              symbol,
              ts::bigint AS ts,
              composite::float8 AS comp
            FROM xsec_signals
            WHERE lower(exchange) = lower($1)
              AND market_type = $2
              AND symbol = ANY($3)
              AND ts >= (EXTRACT(EPOCH FROM (NOW() - ($4 * INTERVAL '1 day'))) * 1000)::bigint
            ORDER BY symbol, ts DESC
            "#,
        )
        .bind(&q.exchange)
        .bind(mt)
        .bind(syms)
        .bind(since_days as f64)
        .fetch_all(&pool)
        .await
        .map_err(internal)?
    } else {
        sqlx::query(
            r#"
            SELECT DISTINCT ON (symbol)
              symbol,
              ts::bigint AS ts,
              composite::float8 AS comp
            FROM xsec_signals
            WHERE lower(exchange) = lower($1)
              AND market_type = $2
              AND ts >= (EXTRACT(EPOCH FROM (NOW() - ($3 * INTERVAL '1 day'))) * 1000)::bigint
            ORDER BY symbol, ts DESC
            "#,
        )
        .bind(&q.exchange)
        .bind(mt)
        .bind(since_days as f64)
        .fetch_all(&pool)
        .await
        .map_err(internal)?
    };

    // Build rows and apply mapping / constraints
    let mut out_rows: Vec<PortfolioRow> = Vec::with_capacity(rows.len());
    let mut gross_before = 0.0;

    for r in rows {
        let symbol: String = r.get::<String, _>("symbol");
        let ts: i64 = r.get::<i64, _>("ts");
        let comp: f64 = r.get::<f64, _>("comp");
        if !comp.is_finite() {
            continue;
        }

        let exp_raw = sigmoid(comp, mapping); // [-cap,+cap] around center + bias
        let exp_capped = clip(exp_raw, -constraints.max_pos, constraints.max_pos);
        gross_before += exp_capped.abs();

        out_rows.push(PortfolioRow {
            symbol,
            ts,
            signal: comp,
            exposure_raw: exp_raw,
            exposure_capped: exp_capped,
            exposure_final: 0.0, // fill after gross scaling
        });
    }

    // Gross scaling to gross_cap
    let scale = if gross_before > constraints.gross_cap && gross_before > 0.0 {
        constraints.gross_cap / gross_before
    } else {
        1.0
    };

    let mut gross_after = 0.0;
    for row in &mut out_rows {
        row.exposure_final = row.exposure_capped * scale;
        gross_after += row.exposure_final.abs();
    }

    Ok(Json(PortfolioResponse {
        exchange: q.exchange,
        market_type: mt.to_string(),
        variant_id: q.variant_id,
        params: PortfolioParamsOut {
            mapping,
            constraints,
        },
        counts: PortfolioCounts {
            symbols: out_rows.len(),
            with_signal: out_rows.len(),
        },
        gross_before,
        gross_after,
        rows: out_rows,
    }))
}

// ========================
// PUT /trend/portfolio/constraints
// ========================

pub async fn put_constraints(
    State(pool): State<PgPool>,
    Json(mut body): Json<UpsertConstraints>,
) -> Result<Json<ConstraintsSaved>, (StatusCode, String)> {
    // Ensure we have a variant_id; if not, create one
    let variant_id = match (body.variant_id, body.name.clone()) {
        (Some(id), _) => id,
        (None, Some(name)) => {
            // upsert by name; return id
            let rec = sqlx::query!(
                r#"
                INSERT INTO portfolio_variants (name)
                VALUES ($1)
                ON CONFLICT (name) DO UPDATE SET updated_at = NOW()
                RETURNING id
                "#,
                name
            )
            .fetch_one(&pool)
            .await
            .map_err(internal)?;
            rec.id
        }
        (None, None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "variant_id or name is required".into(),
            ));
        }
    };

    // Defaults; only overwrite if provided
    let existing = load_constraints(&pool, variant_id).await.ok();

    // =================================================================================
    // FIX: The original code had type mismatches and an ownership bug.
    // The logic is to take the value from the `body` if it exists, otherwise
    // fall back to the `existing` value from the database.
    // We use `or` for required fields (mapping the inner value out of `existing`)
    // and `or_else` for optional fields (to flatten the nested Options).
    // `as_ref()` is used on `existing` to avoid moving it, which would cause a compile error.
    // =================================================================================
    let gross_cap = body
        .gross_cap
        .or(existing.as_ref().map(|c| c.gross_cap))
        .unwrap_or(1.0);
    let max_pos = body
        .max_pos
        .or(existing.as_ref().map(|c| c.max_pos))
        .unwrap_or(0.10);
    let turnover_cap = body
        .turnover_cap
        .or_else(|| existing.as_ref().and_then(|c| c.turnover_cap));
    let vol_target = body
        .vol_target
        .or_else(|| existing.as_ref().and_then(|c| c.vol_target));
    let lambda_risk = body
        .lambda_risk
        .or_else(|| existing.as_ref().and_then(|c| c.lambda_risk));
    let gamma_cost = body
        .gamma_cost
        .or_else(|| existing.as_ref().and_then(|c| c.gamma_cost));
    let ignore_open = body
        .ignore_open
        .or(existing.as_ref().map(|c| c.ignore_open))
        .unwrap_or(false);

    // Upsert constraints
    let _ = sqlx::query!(
        r#"
        INSERT INTO variant_constraints
            (variant_id, gross_cap, max_pos, turnover_cap, vol_target, lambda_risk, gamma_cost, ignore_open, updated_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        ON CONFLICT (variant_id) DO UPDATE SET
            gross_cap    = EXCLUDED.gross_cap,
            max_pos      = EXCLUDED.max_pos,
            turnover_cap = EXCLUDED.turnover_cap,
            vol_target   = EXCLUDED.vol_target,
            lambda_risk  = EXCLUDED.lambda_risk,
            gamma_cost   = EXCLUDED.gamma_cost,
            ignore_open  = EXCLUDED.ignore_open,
            updated_at   = NOW()
        "#,
        variant_id,
        gross_cap,
        max_pos,
        turnover_cap,
        vol_target,
        lambda_risk,
        gamma_cost,
        ignore_open
    )
    .execute(&pool).await.map_err(internal)?;

    Ok(Json(ConstraintsSaved {
        variant_id,
        constraints: ConstraintsOut {
            gross_cap,
            max_pos,
            turnover_cap,
            vol_target,
            lambda_risk,
            gamma_cost,
            ignore_open,
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct ConstraintsSaved {
    pub variant_id: i32,
    pub constraints: ConstraintsOut,
}

// ========================
// Helpers
// ========================

fn parse_market_type(s: &str) -> Result<&'static str, String> {
    match s {
        "spot" => Ok("spot"),
        "perps" => Ok("perps"),
        other => Err(format!(
            "invalid market_type '{}'; use 'spot' or 'perps'",
            other
        )),
    }
}

fn sigmoid(x: f64, p: SigmoidParams) -> f64 {
    // y = bias + cap * ( 2 / (1 + exp(-slope*(x - center))) - 1 )
    let t = (-p.slope * (x - p.center)).exp();
    p.bias + p.cap * ((2.0 / (1.0 + t)) - 1.0)
}

fn clip(x: f64, lo: f64, hi: f64) -> f64 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

async fn load_constraints(pool: &PgPool, variant_id: i32) -> anyhow::Result<ConstraintsOut> {
    let r = sqlx::query(
        r#"
        SELECT gross_cap, max_pos, turnover_cap, vol_target, lambda_risk, gamma_cost, ignore_open
        FROM variant_constraints
        WHERE variant_id = $1
        "#,
    )
    .bind(variant_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = r {
        Ok(ConstraintsOut {
            gross_cap: row.get::<f64, _>("gross_cap"),
            max_pos: row.get::<f64, _>("max_pos"),
            turnover_cap: row.try_get::<Option<f64>, _>("turnover_cap").ok().flatten(),
            vol_target: row.try_get::<Option<f64>, _>("vol_target").ok().flatten(),
            lambda_risk: row.try_get::<Option<f64>, _>("lambda_risk").ok().flatten(),
            gamma_cost: row.try_get::<Option<f64>, _>("gamma_cost").ok().flatten(),
            ignore_open: row
                .try_get::<Option<bool>, _>("ignore_open")
                .ok()
                .flatten()
                .unwrap_or(false),
        })
    } else {
        anyhow::bail!("no constraints for variant_id={}", variant_id)
    }
}
