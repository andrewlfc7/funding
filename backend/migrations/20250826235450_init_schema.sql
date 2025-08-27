-- Add migration script here
DROP VIEW IF EXISTS funding_matrix_view;
DROP VIEW IF EXISTS latest_market_stats_view;
DROP VIEW IF EXISTS latest_funding_rates_view;

DROP TABLE IF EXISTS market_stats;
DROP TABLE IF EXISTS funding_rates;
DROP TABLE IF EXISTS markets;
DROP TABLE IF EXISTS tokens;
DROP TABLE IF EXISTS exchanges;


-- ---------- Tables ----------
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
    id BIGSERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchanges(id) ON DELETE CASCADE,
    market_id INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    rate NUMERIC(18,10) NOT NULL,
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

-- ---------- Base “latest” views (must be before matrix view) ----------
CREATE OR REPLACE VIEW latest_funding_rates_view AS
SELECT DISTINCT ON (fr.market_id)
    fr.market_id,
    fr.rate,
    fr.timestamp
FROM funding_rates fr
ORDER BY fr.market_id, fr.timestamp DESC;

CREATE OR REPLACE VIEW latest_market_stats_view AS
SELECT DISTINCT ON (ms.market_id)
    ms.market_id,
    ms.open_interest,
    ms.volume_24h,
    ms.timestamp
FROM market_stats ms
ORDER BY ms.market_id, ms.timestamp DESC;

-- ---------- Matrix view w/ OI ----------
CREATE OR REPLACE VIEW funding_matrix_view AS
SELECT
  t.symbol,
  jsonb_object_agg(
    e.name,
    jsonb_build_object(
      'funding_rate',  COALESCE(lfr.rate::float8, 0.0),
      'open_interest', COALESCE(lms.open_interest::float8, 0.0)
    )
    ORDER BY e.name
  ) AS rates,
  MAX(
    GREATEST(
      COALESCE(lfr.timestamp, 'epoch'::timestamptz),
      COALESCE(lms.timestamp, 'epoch'::timestamptz)
    )
  ) AS last_update
FROM markets m
JOIN tokens t    ON m.token_id    = t.id
JOIN exchanges e ON m.exchange_id = e.id
LEFT JOIN latest_funding_rates_view lfr ON m.id = lfr.market_id
LEFT JOIN latest_market_stats_view  lms ON m.id = lms.market_id
WHERE m.is_active = true
  AND t.symbol IS NOT NULL
GROUP BY t.symbol;
