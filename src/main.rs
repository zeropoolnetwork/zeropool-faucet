use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{ConnectInfo, State},
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
        .route("/near/:address", post(near))
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
        Ok(Self {
            config: config.clone(),
            addresses: Arc::new(RwLock::new(TtlCache::new(config.server.interval))),
            ips: Arc::new(RwLock::new(TtlCache::new(config.server.interval))),
            near_client: Arc::new(NearClient::new(&config.near)?),
        })
    }
}

async fn near(
    state: State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    to: String,
) -> Result<(), AppError> {
    if state.ips.read().await.contains(&addr.ip()) || state.addresses.read().await.contains(&to) {
        tracing::info!("Address {}, {} tried to request too often", addr, to);
        return Err(AppError::TooManyRequests);
    }

    state
        .near_client
        .transfer(&to, state.config.near.amount)
        .await?;

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
