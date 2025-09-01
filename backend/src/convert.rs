// src/convert.rs
use crate::utils::cli::CliMarketType;
use crate::cex::common::CexMarketType;

impl From<CliMarketType> for CexMarketType {
    fn from(cli: CliMarketType) -> Self {
        match cli {
            CliMarketType::Spot => Self::Spot,
            CliMarketType::Perps => Self::Perps,
        }
    }
}
