use crate::exchanges::paradex::api::{
    client::ParadexClient,
    endpoints::ApiEnvironment as ParadexEnv,
};
use crate::exchanges::extended::api::{
    client::ExtendedClient,
    endpoints::ApiEnvironment as ExtendedEnv,
};


use crate::exchanges::paradex::handler::handler::parse_paradex_markets;
use crate::exchanges::extended::handler::handler::parse_extended_markets;
use crate::db::insert::upsert_markets;
use sqlx::PgPool;



pub async fn refresh_all_markets(pool: &PgPool) -> anyhow::Result<()> {
    let exchanges = sqlx::query!("SELECT id,name FROM exchanges WHERE is_active=true")
        .fetch_all(pool).await?;

    for exch in exchanges {
        match exch.name.as_str() {
            "paradex" => {
                let client = ParadexClient::new(ParadexEnv::Mainnet);
                let raw = client.get_markets().await?;
                let markets = parse_paradex_markets(&raw)?;
                upsert_markets(pool, exch.id, &markets).await?;
            }
            "extended" => {
                let client = ExtendedClient::new(ExtendedEnv::Mainnet);
                let raw = client.get_markets(None).await?;
                let markets = parse_extended_markets(&raw)?;
                upsert_markets(pool, exch.id, &markets).await?;
            }
            _ => {}
        }
    }
    Ok(())
}



