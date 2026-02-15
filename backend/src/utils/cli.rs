use crate::exchanges::shared::time::TimeSpec;
use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
#[clap(rename_all = "kebab_case")]
pub enum CliMarketType {
    Spot,
    Perps,
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
#[clap(rename_all = "kebab_case")]
pub enum CliSource {
    Klines,
    Trades,
    Both,
}

#[derive(Parser, Debug)]
#[command(name = "sync", about = "sync CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Sync {
        #[command(subcommand)]
        workflow: Workflow,
    },

    CexAdd {
        #[clap(long, short)]
        name: String,

        #[clap(long, value_enum)]
        market_type: CliMarketType,

        #[clap(long, value_parser = ["trend", "zscore"], default_value = "trend")]
        workflow: String,

        #[clap(flatten)]
        time_spec: CliTimeSpec,

        #[clap(long)]
        quote: Option<String>,

        #[clap(long)]
        all_quotes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum Workflow {
    Funding {
        #[clap(long)]
        exchange: Option<String>,

        #[clap(flatten)]
        time_spec: CliTimeSpec,
    },

    Trend {
        #[clap(long)]
        exchange: String,

        #[clap(long, value_enum)]
        market_type: CliMarketType,

        #[clap(flatten)]
        time_spec: CliTimeSpec,

        #[clap(long, value_enum, default_value = "klines")]
        source: CliSource,

        #[clap(long)]
        quote: Option<String>,

        #[clap(long)]
        all_quotes: bool,
    },

    Zscore {
        #[clap(long)]
        exchange: String,

        #[clap(long, value_enum)]
        market_type: CliMarketType,

        #[clap(flatten)]
        time_spec: CliTimeSpec,

        #[clap(long, value_enum, default_value = "both")]
        source: CliSource,

        #[clap(long)]
        quote: Option<String>,

        #[clap(long)]
        all_quotes: bool,
    },
}

#[derive(Args, Debug)]
pub struct CliTimeSpec {
    #[clap(long, conflicts_with = "days")]
    pub hours: Option<u64>,

    #[clap(long, conflicts_with = "hours")]
    pub days: Option<u64>,

    #[clap(long, num_args = 2, value_names = ["START_MS", "END_MS"])]
    pub between: Option<Vec<u64>>,

    #[clap(long, conflicts_with = "since_last_days")]
    pub since_last: Option<u64>,

    #[clap(long, conflicts_with = "since_last")]
    pub since_last_days: Option<u64>,
}

impl CliTimeSpec {
    pub fn to_time_spec(&self) -> Result<TimeSpec> {
        if let Some(h) = self
            .hours
            .or_else(|| self.days.map(|d| d.saturating_mul(24)))
        {
            return Ok(TimeSpec::LookbackHours(h));
        }
        if let Some(b) = &self.between {
            if b.len() == 2 {
                return Ok(TimeSpec::Between {
                    start_ms: b[0],
                    end_ms: b[1],
                });
            } else {
                return Err(anyhow::anyhow!("--between requires START_MS END_MS"));
            }
        }
        if let Some(h) = self
            .since_last
            .or_else(|| self.since_last_days.map(|d| d.saturating_mul(24)))
        {
            return Ok(TimeSpec::SinceLastOrLookbackHours(h));
        }
        Ok(TimeSpec::SinceLastOrLookbackHours(24))
    }
}
