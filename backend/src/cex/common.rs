use anyhow::anyhow;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CexMarketType {
    Spot,
    Perps,
}

impl FromStr for CexMarketType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "spot" => Ok(CexMarketType::Spot),
            "perps" | "perp" => Ok(CexMarketType::Perps),
            _ => Err(anyhow!(
                "Invalid CEX market type: '{}'. Use 'spot' or 'perps'.",
                s
            )),
        }
    }
}

impl CexMarketType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CexMarketType::Spot => "spot",
            CexMarketType::Perps => "perps",
        }
    }
}
