-- Enable TimescaleDB
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- ---------- Clean drop (views first to avoid deps) ----------
DROP VIEW IF EXISTS funding_matrix_view;
DROP VIEW IF EXISTS latest_funding_8h_view;
DROP VIEW IF EXISTS funding_8h_view;
DROP VIEW IF EXISTS latest_market_stats_view;

-- ---------- Drop base tables (idempotent) ----------
DROP TABLE IF EXISTS market_stats;
DROP TABLE IF EXISTS funding_rates;
DROP TABLE IF EXISTS markets;
DROP TABLE IF EXISTS tokens;
DROP TABLE IF EXISTS exchanges;

-- ---------- Core dimension tables ----------
CREATE TABLE exchanges (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    funding_interval_minutes INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tokens (
    id SERIAL PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE markets (
    id SERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchanges(id) ON DELETE CASCADE,
    token_id INTEGER NOT NULL REFERENCES tokens(id) ON DELETE CASCADE,
    market_symbol TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(exchange_id, market_symbol)
);

CREATE TABLE funding_rates (
    exchange_id INTEGER NOT NULL REFERENCES exchanges(id) ON DELETE CASCADE,
    market_id   INTEGER NOT NULL REFERENCES markets(id)   ON DELETE CASCADE,
    rate        NUMERIC(18,10) NOT NULL,
    "timestamp" TIMESTAMPTZ    NOT NULL,
    PRIMARY KEY (market_id, "timestamp")
);

SELECT create_hypertable(
  'funding_rates', 'timestamp',
  chunk_time_interval => INTERVAL '30 days',
  if_not_exists => TRUE
);


CREATE INDEX idx_funding_rates_on_exchange_id_timestamp_desc
  ON funding_rates(exchange_id, "timestamp" DESC);


CREATE TABLE market_stats (
    market_id     INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    open_interest NUMERIC(60,30),
    volume_24h    NUMERIC(60,30),
    "timestamp"   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (market_id, "timestamp")
);

SELECT create_hypertable(
  'market_stats', 'timestamp',
  chunk_time_interval => INTERVAL '7 days',
  if_not_exists => TRUE
);

CREATE INDEX idx_market_stats_market_timestamp
  ON market_stats(market_id, "timestamp" DESC);

-- ---------- Convenience indexes on dimensions ----------
CREATE INDEX idx_markets_on_exchange_id ON markets(exchange_id);
CREATE INDEX idx_markets_on_token_id    ON markets(token_id);

-- ---------- Trigger to normalize exchange name ----------
CREATE OR REPLACE FUNCTION normalize_exchange_name()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  NEW.name := initcap(lower(NEW.name));
  RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_exchanges_name_normalize ON exchanges;
CREATE TRIGGER trg_exchanges_name_normalize
BEFORE INSERT OR UPDATE ON exchanges
FOR EACH ROW
EXECUTE FUNCTION normalize_exchange_name();

-- ---------- Latest market stats per market (carry exchange_id) ----------
CREATE OR REPLACE VIEW latest_market_stats_view AS
WITH ranked AS (
  SELECT
    m.exchange_id,
    ms.market_id,
    ms.open_interest,
    ms.volume_24h,
    ms."timestamp",
    ROW_NUMBER() OVER (
      PARTITION BY ms.market_id
      ORDER BY ms."timestamp" DESC
    ) AS rn
  FROM market_stats ms
  JOIN markets m ON m.id = ms.market_id
)
SELECT exchange_id, market_id, open_interest, volume_24h, "timestamp"
FROM ranked
WHERE rn = 1;

-- ---------- Step 1: compute average observed funding interval per (exchange_id, market_id) ----------
CREATE OR REPLACE VIEW funding_intervals_view AS
WITH lagged_timestamps AS (
  SELECT
    fr.exchange_id,
    fr.market_id,
    fr."timestamp",
    LAG(fr."timestamp") OVER (
      PARTITION BY fr.exchange_id, fr.market_id 
      ORDER BY fr."timestamp"
    ) AS prev_timestamp
  FROM funding_rates fr
)
SELECT
  exchange_id,
  market_id,
  AVG(EXTRACT(EPOCH FROM ("timestamp" - prev_timestamp)) / 3600.0) AS avg_interval_hours
FROM lagged_timestamps
WHERE prev_timestamp IS NOT NULL  -- exclude first row in each partition
GROUP BY exchange_id, market_id;



CREATE OR REPLACE VIEW funding_8h_view AS
WITH ordered AS (
  SELECT
    fr.exchange_id,
    fr.market_id,
    fr.rate,
    fr."timestamp",
    LEAD(fr."timestamp") OVER (
      PARTITION BY fr.exchange_id, fr.market_id 
      ORDER BY fr."timestamp"
    ) AS next_timestamp
  FROM funding_rates fr
),
weighted AS (
  SELECT
    o.exchange_id,
    o.market_id,
    (
      date_trunc('hour', o."timestamp")
      - ((EXTRACT(HOUR FROM o."timestamp")::int % 8) * interval '1 hour')
    ) AS bucket_start,
    o.rate,
    EXTRACT(EPOCH FROM (
      COALESCE(o.next_timestamp, o."timestamp" + interval '1 hour') - o."timestamp"
    )) / 3600.0 AS hours_held
  FROM ordered o
)
SELECT
  w.exchange_id,
  w.market_id,
  w.bucket_start,
  -- weighted-average of rate over coverage time, normalized to 8h
  SUM(w.rate * w.hours_held) / 8.0 AS rate_8h,
  COUNT(*) AS samples
FROM weighted w
GROUP BY w.exchange_id, w.market_id, w.bucket_start;




-- ---------- Step 3: latest normalized 8h funding ----------
CREATE OR REPLACE VIEW latest_funding_8h_view AS
WITH ranked AS (
  SELECT
    f8.exchange_id,
    f8.market_id,
    f8.rate_8h,
    f8.bucket_start AS "timestamp",
    ROW_NUMBER() OVER (
      PARTITION BY f8.exchange_id, f8.market_id
      ORDER BY f8.bucket_start DESC
    ) AS rn
  FROM funding_8h_view f8
)
SELECT exchange_id, market_id, rate_8h, "timestamp"
FROM ranked
WHERE rn = 1;


-- ---------- Step 4: frontend‑ready matrix (token x exchange) ----------
CREATE OR REPLACE VIEW funding_matrix_view AS
SELECT
  t.symbol,
  jsonb_object_agg(
    e.name,
    jsonb_build_object(
      'market_symbol',   m.market_symbol,
      'funding_rate_8h', lfr.rate_8h::float8,
      'funding_bucket',  lfr."timestamp",
      'open_interest',   lms.open_interest,
      'volume_24h',      lms.volume_24h,
      'stats_ts',        lms."timestamp"
    )
    ORDER BY e.name
  ) AS per_exchange,
  MAX(
    GREATEST(
      COALESCE(lfr."timestamp", 'epoch'::timestamptz),
      COALESCE(lms."timestamp", 'epoch'::timestamptz)
    )
  ) AS last_update
FROM markets m
JOIN tokens    t ON t.id = m.token_id
JOIN exchanges e ON e.id = m.exchange_id
LEFT JOIN latest_funding_8h_view lfr
  ON lfr.exchange_id = e.id AND lfr.market_id = m.id
LEFT JOIN latest_market_stats_view lms
  ON lms.exchange_id = e.id AND lms.market_id = m.id
WHERE m.is_active = true
GROUP BY t.symbol;

