use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{ConnectInfo, Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tokio::sync::RwLock;

use crate::{cache::TtlCache, config::Config, error::*, near::NearClient};

mod cache;
mod config;
mod error;
mod near;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::init();

    let app = Router::new()
        .route("/near/:to", post(near))
        .route("/info", get(info))
        .with_state(AppState::new(&config)?);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    config: Config,
    addresses: Arc<RwLock<TtlCache<String>>>,
    ips: Arc<RwLock<TtlCache<IpAddr>>>,
    near_client: Arc<NearClient>,
}

impl AppState {
    fn new(config: &Config) -> anyhow::Result<Self> {
        let interval = Duration::from_millis(config.server.interval);
        Ok(Self {
            config: config.clone(),
            addresses: Arc::new(RwLock::new(TtlCache::new(interval))),
            ips: Arc::new(RwLock::new(TtlCache::new(interval))),
            near_client: Arc::new(NearClient::new(&config.near)?),
        })
    }
}

async fn near(
    state: State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(to): Path<String>,
) -> Result<(), AppError> {
    tracing::debug!("{}", to);

    if state.ips.read().await.contains(&addr.ip()) || state.addresses.read().await.contains(&to) {
        tracing::info!("Address {}, {} tried to request too often", addr, to);
        return Err(AppError::TooManyRequests);
    }

    let amount = state.config.near.amount.parse()?;

    state.near_client.transfer(&to, amount).await?;

    tracing::debug!("Updating cache for {} {}", addr.ip(), to);
    state.ips.write().await.add(addr.ip());
    state.addresses.write().await.add(to);

    Ok(())
}

async fn info() -> impl IntoResponse {
    #[derive(Serialize)]
    struct Info {
        version: &'static str,
        supported_networks: &'static [&'static str],
    }

    Json(Info {
        version: env!("CARGO_PKG_VERSION"),
        supported_networks: &["near"],
    })
}
