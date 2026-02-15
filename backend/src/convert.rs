// src/convert.rs
use crate::cex::common::CexMarketType;
use crate::utils::cli::CliMarketType;

impl From<CliMarketType> for CexMarketType {
    fn from(cli: CliMarketType) -> Self {
        match cli {
            CliMarketType::Spot => Self::Spot,
            CliMarketType::Perps => Self::Perps,
        }
    }
}
