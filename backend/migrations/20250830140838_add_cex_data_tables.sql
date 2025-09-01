-- =====================================
-- CEX Exchanges Table
-- =====================================
CREATE TABLE cex_exchanges (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,              -- e.g. "Binance", "Bybit"
    description TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- =====================================
-- Markets Table (for CEXes specifically)
-- =====================================
CREATE TABLE cex_markets (
    id SERIAL PRIMARY KEY,
    exchange_id INT NOT NULL REFERENCES cex_exchanges(id) ON DELETE CASCADE,
    
    symbol TEXT NOT NULL,
    
    market_symbol TEXT NOT NULL,
    
    base_asset TEXT NOT NULL,
    quote_asset TEXT NOT NULL,
    market_type TEXT NOT NULL CHECK (market_type IN ('spot', 'perps')),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(exchange_id, market_symbol, market_type)
);


-- =====================================
CREATE TABLE klines_daily (
    market_id INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    open NUMERIC(20, 10) NOT NULL,
    high NUMERIC(20, 10) NOT NULL,
    low NUMERIC(20, 10) NOT NULL,
    close NUMERIC(20, 10) NOT NULL,
    volume NUMERIC(30, 10) NOT NULL,
    PRIMARY KEY (market_id, date)
);

-- =====================================
-- Klines (Hourly)
--
-- Note: The open_time column remains a TIMESTAMP WITH TIME ZONE.
-- This is necessary to ensure each hourly entry for a given
-- market is unique and can serve as a primary key.
-- =====================================
CREATE TABLE klines_hourly (
    market_id INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
    time TIMESTAMP WITH TIME ZONE NOT NULL,
    open NUMERIC(20, 10) NOT NULL,
    high NUMERIC(20, 10) NOT NULL,
    low NUMERIC(20, 10) NOT NULL,
    close NUMERIC(20, 10) NOT NULL,
    volume NUMERIC(30, 10) NOT NULL,
    PRIMARY KEY (market_id, time)
);



-- =====================================
-- Trades
-- =====================================
CREATE TABLE trades (
    market_id INT NOT NULL REFERENCES cex_markets(id) ON DELETE CASCADE,
    trade_id TEXT NOT NULL,
    trade_time TIMESTAMP WITH TIME ZONE NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
    price NUMERIC(20, 10) NOT NULL,
    qty NUMERIC(30, 10) NOT NULL,
    quote_qty NUMERIC(30, 10),
    PRIMARY KEY (market_id, trade_id)
);

-- =====================================
-- Indexes for fast sync / lookups
-- =====================================

CREATE INDEX idx_klines_daily_lookup 
    ON klines_daily (market_id, date DESC);

CREATE INDEX idx_klines_hourly_lookup 
    ON klines_hourly (market_id, time DESC);


CREATE INDEX idx_trades_lookup 
    ON trades (market_id, trade_time DESC);
    