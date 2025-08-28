
DROP TABLE IF EXISTS market_stats;
DROP TABLE IF EXISTS funding_rates;
DROP TABLE IF EXISTS markets;
DROP TABLE IF EXISTS tokens;
DROP TABLE IF EXISTS exchanges;

-- ---------- Tables ----------
CREATE TABLE exchanges (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,                    -- will be normalized to Capitalized via trigger
    funding_interval_minutes INTEGER,             -- optional; fill later from sync/config
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
    id BIGSERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchanges(id) ON DELETE CASCADE,
    market_id INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    rate NUMERIC(18,10) NOT NULL,                 -- store as fraction; UI can show bps
    timestamp TIMESTAMPTZ NOT NULL,
    UNIQUE(market_id, timestamp)
);

CREATE TABLE market_stats (
    id BIGSERIAL PRIMARY KEY,
    market_id INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    open_interest NUMERIC(30,10),
    volume_24h NUMERIC(30,10),
    timestamp TIMESTAMPTZ NOT NULL,
    UNIQUE(market_id, timestamp)
);

-- ---------- Indexes ----------
CREATE INDEX idx_markets_on_exchange_id ON markets(exchange_id);
CREATE INDEX idx_markets_on_token_id ON markets(token_id);
CREATE INDEX idx_funding_rates_on_market_id_timestamp_desc ON funding_rates(market_id, timestamp DESC);
CREATE INDEX idx_funding_rates_on_exchange_id_timestamp_desc ON funding_rates(exchange_id, timestamp DESC);
CREATE INDEX idx_market_stats_market_timestamp ON market_stats(market_id, timestamp DESC);


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


-- ---------- Clean drop (views only) ----------
DROP VIEW IF EXISTS funding_matrix_view;
DROP VIEW IF EXISTS latest_funding_8h_view;
DROP VIEW IF EXISTS funding_8h_view;
DROP VIEW IF EXISTS latest_market_stats_view;

-- ---------- Latest market stats per market (carry exchange_id) ----------
CREATE OR REPLACE VIEW latest_market_stats_view AS
WITH ranked AS (
  SELECT
    m.exchange_id,
    ms.market_id,
    ms.open_interest,
    ms.volume_24h,
    ms.timestamp,
    ROW_NUMBER() OVER (
      PARTITION BY ms.market_id
      ORDER BY ms.timestamp DESC
    ) AS rn
  FROM market_stats ms
  JOIN markets m ON m.id = ms.market_id
)
SELECT exchange_id, market_id, open_interest, volume_24h, timestamp
FROM ranked
WHERE rn = 1;

-- ---------- Normalize funding into 8h buckets ----------
-- Bucket start = top-of-hour minus (hour % 8) hours
CREATE OR REPLACE VIEW funding_8h_view AS
SELECT
  fr.exchange_id,
  fr.market_id,
  (date_trunc('hour', fr.timestamp)
   - ( (EXTRACT(HOUR FROM fr.timestamp)::int % 8) * interval '1 hour')
  ) AS bucket_start,
  AVG(fr.rate) AS rate_8h,           -- simple agg over raw ticks in the 8h window
  COUNT(*)     AS samples            -- useful for debugging/quality
FROM funding_rates fr
GROUP BY fr.exchange_id, fr.market_id, bucket_start;

-- ---------- Latest 8h funding per (exchange_id, market_id) ----------
CREATE OR REPLACE VIEW latest_funding_8h_view AS
WITH ranked AS (
  SELECT
    f8.exchange_id,
    f8.market_id,
    f8.rate_8h,
    f8.bucket_start AS timestamp,
    ROW_NUMBER() OVER (
      PARTITION BY f8.exchange_id, f8.market_id
      ORDER BY f8.bucket_start DESC
    ) AS rn
  FROM funding_8h_view f8
)
SELECT exchange_id, market_id, rate_8h, timestamp
FROM ranked
WHERE rn = 1;

-- ---------- Frontend-ready matrix (per token, per exchange) ----------
CREATE OR REPLACE VIEW funding_matrix_view AS
SELECT
  t.symbol,
  jsonb_object_agg(
    e.name,
    jsonb_build_object(
      'market_symbol',     m.market_symbol,
      'funding_rate_8h',   (lfr.rate_8h)::float8,
      'funding_bucket',    lfr.timestamp,
      'open_interest',     (lms.open_interest)::float8,
      'volume_24h',        (lms.volume_24h)::float8,
      'stats_ts',          lms.timestamp
    )
    ORDER BY e.name
  ) AS per_exchange,
  MAX(
    GREATEST(
      COALESCE(lfr.timestamp, 'epoch'::timestamptz),
      COALESCE(lms.timestamp, 'epoch'::timestamptz)
    )
  ) AS last_update
FROM markets m
JOIN tokens    t ON t.id = m.token_id
JOIN exchanges e ON e.id = m.exchange_id
LEFT JOIN latest_funding_8h_view lfr
  ON lfr.exchange_id = e.id AND lfr.market_id = m.id
LEFT JOIN latest_market_stats_view  lms
  ON lms.exchange_id = e.id AND lms.market_id = m.id
WHERE m.is_active = true
GROUP BY t.symbol;
