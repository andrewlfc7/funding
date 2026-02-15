
-- =======================================================================
-- CEX schema (kept as you had it; hypertables only where keys include time)
-- =======================================================================

CREATE TABLE IF NOT EXISTS cex_exchanges (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cex_markets (
    id SERIAL PRIMARY KEY,
    exchange_id INT NOT NULL REFERENCES cex_exchanges(id) ON DELETE CASCADE,
    symbol TEXT NOT NULL,
    market_symbol TEXT NOT NULL,
    base_asset TEXT NOT NULL,
    quote_asset TEXT NOT NULL,
    market_type TEXT NOT NULL CHECK (market_type IN ('spot', 'perps')),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(exchange_id, market_symbol, market_type)
);

-- Daily bars (PK includes partition key -> safe to hypertable)
CREATE TABLE IF NOT EXISTS klines_daily (
    market_id INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
    "date" DATE NOT NULL,
    open  NUMERIC(20, 10) NOT NULL,
    high  NUMERIC(20, 10) NOT NULL,
    low   NUMERIC(20, 10) NOT NULL,
    close NUMERIC(20, 10) NOT NULL,
    volume NUMERIC(30, 10) NOT NULL,
    PRIMARY KEY (market_id, "date")
);

-- Hourly bars (PK includes partition key -> safe to hypertable)
CREATE TABLE IF NOT EXISTS klines_hourly (
    market_id INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
    "time" TIMESTAMPTZ NOT NULL,
    open  NUMERIC(20, 10) NOT NULL,
    high  NUMERIC(20, 10) NOT NULL,
    low   NUMERIC(20, 10) NOT NULL,
    close NUMERIC(20, 10) NOT NULL,
    volume NUMERIC(30, 10) NOT NULL,
    PRIMARY KEY (market_id, "time")
);

-- Trades (KEEP AS REGULAR TABLE to match your current ON CONFLICT (market_id, trade_id))
CREATE TABLE IF NOT EXISTS trades (
    market_id INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
    trade_id TEXT NOT NULL,
    trade_time TIMESTAMPTZ NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
    price NUMERIC(20, 10) NOT NULL,
    qty NUMERIC(30, 10) NOT NULL,
    quote_qty NUMERIC(30, 10),
    PRIMARY KEY (market_id, trade_id)
);

-- Hypertables for CEX series whose PK includes the time column
SELECT create_hypertable('klines_daily',  'date', chunk_time_interval => INTERVAL '90 days', if_not_exists => TRUE);
SELECT create_hypertable('klines_hourly', 'time', chunk_time_interval => INTERVAL '14 days', if_not_exists => TRUE);
-- NOTE: trades left as regular table; if you want hypertable, change PK to (market_id, trade_time, trade_id).

-- Indexes for fast sync / lookups
CREATE INDEX IF NOT EXISTS idx_klines_daily_lookup  ON klines_daily  (market_id, "date" DESC);
CREATE INDEX IF NOT EXISTS idx_klines_hourly_lookup ON klines_hourly (market_id, "time" DESC);
CREATE INDEX IF NOT EXISTS idx_trades_lookup        ON trades        (market_id, trade_time DESC);
