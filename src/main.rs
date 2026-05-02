use std::net::SocketAddr;

use typenx_addon_myanimelist::{api::MyAnimeListCatalog, server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "typenx_addon_myanimelist=info,tower_http=info".into()),
        )
        .init();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8787);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let catalog = MyAnimeListCatalog::new(std::env::var("MAL_CLIENT_ID").ok());

    server::serve(addr, catalog).await
}
