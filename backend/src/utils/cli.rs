use clap::{Parser, Subcommand, Args, ValueEnum};
use anyhow::Result;
use crate::exchanges::shared::time::TimeSpec;

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
#[clap(rename_all = "kebab_case")] // accepts "spot", "perps"
pub enum CliMarketType { Spot, Perps }

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
#[clap(rename_all = "kebab_case")] // accepts "klines", "trades", "both"
pub enum CliSource { Klines, Trades, Both }

#[derive(Parser, Debug)]
#[command(name = "sync", about = "sync CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// All top-level commands
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run predefined workflows (funding, trend, zscore)
    Sync { #[command(subcommand)] workflow: Workflow },

    /// Add a CEX exchange and run an initial sync
    CexAdd {
        /// Exchange name, e.g. Binance
        #[clap(long, short)]
        name: String,

        /// Spot or Perps
        #[clap(long, value_enum)]
        market_type: CliMarketType,

        /// Which workflow to run after adding: "trend" or "zscore"
        #[clap(long, value_parser=["trend", "zscore"], default_value="trend")]
        workflow: String,

        #[clap(flatten)]
        time_spec: CliTimeSpec,

        /// Optional: restrict to a single quote (e.g. USDT). If omitted and --all-quotes not set,
        /// we use CEX_QUOTE or default to USDT (back-compat).
        #[clap(long)]
        quote: Option<String>,

        /// Include all quotes (ignore quote filter)
        #[clap(long)]
        all_quotes: bool,
    },
}

/// Workflows for Sync
#[derive(Subcommand, Debug)]
pub enum Workflow {
    /// DEX funding workflow (markets -> funding -> stats)
    Funding {
        #[clap(long)]
        exchange: Option<String>,

        #[clap(flatten)]
        time_spec: CliTimeSpec,
    },

    /// CEX trend-following workflow
    Trend {
        #[clap(long)]
        exchange: String,

        #[clap(long, value_enum)]
        market_type: CliMarketType,

        #[clap(flatten)]
        time_spec: CliTimeSpec,

        /// Which data to sync for this workflow
        #[clap(long, value_enum, default_value="klines")]
        source: CliSource,

        /// Optional quote filter (e.g. USDT)
        #[clap(long)]
        quote: Option<String>,

        /// Include all quotes (ignore quote filter)
        #[clap(long)]
        all_quotes: bool,
    },

    /// CEX z-score workflow
    Zscore {
        #[clap(long)]
        exchange: String,

        #[clap(long, value_enum)]
        market_type: CliMarketType,

        #[clap(flatten)]
        time_spec: CliTimeSpec,

        /// Which data to sync for this workflow
        #[clap(long, value_enum, default_value="both")]
        source: CliSource,

        /// Optional quote filter (e.g. USDT)
        #[clap(long)]
        quote: Option<String>,

        /// Include all quotes (ignore quote filter)
        #[clap(long)]
        all_quotes: bool,
    },
}

/// TimeSpec wrapper for CLI parsing
#[derive(Args, Debug)]
pub struct CliTimeSpec {
    /// Lookback hours
    #[clap(long)]
    pub hours: Option<u64>,

    /// Between start and end ms
    #[clap(long, num_args=2, value_names=["START_MS","END_MS"])]
    pub between: Option<Vec<u64>>,

    /// Since last or lookback hours
    #[clap(long)]
    pub since_last: Option<u64>,
}

impl CliTimeSpec {
    pub fn to_time_spec(&self) -> Result<TimeSpec> {
        if let Some(h) = self.hours {
            return Ok(TimeSpec::LookbackHours(h));
        }
        if let Some(b) = &self.between {
            if b.len() == 2 {
                return Ok(TimeSpec::Between { start_ms: b[0], end_ms: b[1] });
            } else {
                return Err(anyhow::anyhow!("--between requires START_MS END_MS"));
            }
        }
        if let Some(h) = self.since_last {
            return Ok(TimeSpec::SinceLastOrLookbackHours(h));
        }
        // default fallback: last 24h
        Ok(TimeSpec::SinceLastOrLookbackHours(24))
    }
}
