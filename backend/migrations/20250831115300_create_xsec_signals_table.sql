-- Add migration script here
CREATE TABLE IF NOT EXISTS xsec_signals (
ts          TIMESTAMPTZ NOT NULL,
symbol      TEXT NOT NULL,
exchange    TEXT NOT NULL,
market_type TEXT NOT NULL,
trend       FLOAT8 NOT NULL,
momentum    FLOAT8 NOT NULL,
ewmac       FLOAT8 NOT NULL,
breakout    FLOAT8 NOT NULL,
composite   FLOAT8 NOT NULL,
created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
PRIMARY KEY (ts, symbol, exchange, market_type)
);
CREATE INDEX IF NOT EXISTS xsec_signals_ts_symbol_idx ON xsec_signals(ts DESC, symbol);