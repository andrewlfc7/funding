-- Add migration script here


CREATE TABLE IF NOT EXISTS strategies (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS portfolio_variants (
  id SERIAL PRIMARY KEY,
  strategy_id INT NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
  name TEXT NOT NULL,                 -- e.g., 'default', 'hi-gross'
  is_default BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (strategy_id, name)
);

-- Variant constraints (kept minimal)
CREATE TABLE IF NOT EXISTS variant_constraints (
  variant_id INT PRIMARY KEY REFERENCES portfolio_variants(id) ON DELETE CASCADE,
  max_gross_exposure  DOUBLE PRECISION NOT NULL DEFAULT 1.0,   -- sum |w_i|
  max_single_name_abs DOUBLE PRECISION NOT NULL DEFAULT 0.10,  -- per-asset cap
  max_leverage        DOUBLE PRECISION,
  max_turnover_daily  DOUBLE PRECISION,
  long_cap            DOUBLE PRECISION,
  short_cap           DOUBLE PRECISION,
  updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Variant universe (current membership; Rust updates this at cutovers)
CREATE TABLE IF NOT EXISTS variant_universe (
  variant_id     INT NOT NULL REFERENCES portfolio_variants(id) ON DELETE CASCADE,
  cex_market_id  INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
  is_active      BOOLEAN NOT NULL DEFAULT TRUE,
  PRIMARY KEY (variant_id, cex_market_id)
);
CREATE INDEX IF NOT EXISTS idx_variant_universe_active
  ON variant_universe (variant_id, is_active, cex_market_id);

-- Variant signal weights (used in Rust to combine signals)
CREATE TABLE IF NOT EXISTS variant_signal_weights (
  variant_id INT NOT NULL REFERENCES portfolio_variants(id) ON DELETE CASCADE,
  signal_key TEXT NOT NULL,
  weight     DOUBLE PRECISION NOT NULL,
  PRIMARY KEY (variant_id, signal_key)
);
CREATE INDEX IF NOT EXISTS idx_vsw_variant_sig ON variant_signal_weights(variant_id, signal_key);

-- =============== Signals store =================

-- Generic signal store (trend/ewmac/mom/vol/etc.); you ingest from Rust.
CREATE TABLE IF NOT EXISTS signal_values_cex (
  cex_market_id INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
  ts            TIMESTAMPTZ NOT NULL,
  signal_key    TEXT NOT NULL,
  value         NUMERIC(30,15) NOT NULL,
  PRIMARY KEY (cex_market_id, ts, signal_key)
);
SELECT create_hypertable('signal_values_cex','ts', if_not_exists => TRUE);
CREATE INDEX IF NOT EXISTS idx_signal_values_by_signal_ts
  ON signal_values_cex (signal_key, ts DESC);
CREATE INDEX IF NOT EXISTS idx_signal_values_by_market_ts
  ON signal_values_cex (cex_market_id, ts DESC);

-- Latest signal point per (market, key)
CREATE OR REPLACE VIEW latest_signal_values_cex AS
WITH ranked AS (
  SELECT
    cex_market_id, signal_key, ts, value,
    ROW_NUMBER() OVER (PARTITION BY cex_market_id, signal_key ORDER BY ts DESC) AS rn
  FROM signal_values_cex
)
SELECT cex_market_id, signal_key, ts, value
FROM ranked WHERE rn = 1;

-- =============== Portfolio results =================

-- Engine snapshots per variant & ts.
-- Store BOTH:
--   combined_signal  = Σ_j weight_j * signal_{i,j}  (pre-mapping)
--   target_exposure  = mapped (sigmoid in Rust) & constrained %NAV (post-mapping)
CREATE TABLE IF NOT EXISTS portfolio_snapshots (
  variant_id     INT NOT NULL REFERENCES portfolio_variants(id) ON DELETE CASCADE,
  ts             TIMESTAMPTZ NOT NULL,
  cex_market_id  INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
  -- legacy field for backward compatibility (you may mirror target_exposure here if desired)
  weight         DOUBLE PRECISION,
  -- new fields:
  combined_signal  DOUBLE PRECISION,
  target_exposure  DOUBLE PRECISION,
  -- optional diagnostics:
  exp_return     DOUBLE PRECISION,   -- μ used at this ts (if you run regression)
  risk_scalar    DOUBLE PRECISION,   -- any scaling applied
  PRIMARY KEY (variant_id, cex_market_id, ts)
);
SELECT create_hypertable('portfolio_snapshots','ts', if_not_exists => TRUE);
CREATE INDEX IF NOT EXISTS idx_ps_variant_ts ON portfolio_snapshots (variant_id, ts DESC);

-- Executed positions history (if different from proposed)
CREATE TABLE IF NOT EXISTS positions_executed (
  variant_id     INT NOT NULL REFERENCES portfolio_variants(id) ON DELETE CASCADE,
  ts             TIMESTAMPTZ NOT NULL,
  cex_market_id  INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
  weight         DOUBLE PRECISION NOT NULL,
  PRIMARY KEY (variant_id, cex_market_id, ts)
);
SELECT create_hypertable('positions_executed','ts', if_not_exists => TRUE);
CREATE INDEX IF NOT EXISTS idx_pe_variant_ts ON positions_executed (variant_id, ts DESC);

-- Latest snapshot view (exposes both combined_signal & target_exposure)
CREATE OR REPLACE VIEW portfolio_snapshots_latest AS
WITH ranked AS (
  SELECT
    variant_id, cex_market_id, ts,
    weight, target_exposure, combined_signal, exp_return, risk_scalar,
    ROW_NUMBER() OVER (PARTITION BY variant_id, cex_market_id ORDER BY ts DESC) AS rn
  FROM portfolio_snapshots
)
SELECT
  variant_id, cex_market_id, ts,
  weight, target_exposure, combined_signal, exp_return, risk_scalar
FROM ranked
WHERE rn = 1;

-- Latest executed positions view
CREATE OR REPLACE VIEW positions_executed_latest AS
WITH ranked AS (
  SELECT
    variant_id, cex_market_id, ts, weight,
    ROW_NUMBER() OVER (PARTITION BY variant_id, cex_market_id ORDER BY ts DESC) AS rn
  FROM positions_executed
)
SELECT variant_id, cex_market_id, ts, weight
FROM ranked
WHERE rn = 1;

-- =============== Convenience returns views (optional) =========

-- Daily returns from klines_daily
CREATE OR REPLACE VIEW returns_1d_view AS
SELECT
  d.market_id AS cex_market_id,
  d."date"::timestamptz AS ts,
  (d."close" / NULLIF(LAG(d."close") OVER (PARTITION BY d.market_id ORDER BY d."date"), 0) - 1)::double precision AS ret_1d
FROM klines_daily d;

-- Hourly returns from klines_hourly
CREATE OR REPLACE VIEW returns_1h_view AS
SELECT
  k.market_id AS cex_market_id,
  k."time"    AS ts,
  (k."close" / NULLIF(LAG(k."close") OVER (PARTITION BY k.market_id ORDER BY k."time"), 0) - 1)::double precision AS ret_1h
FROM klines_hourly k;





