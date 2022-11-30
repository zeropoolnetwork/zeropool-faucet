use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use tokio::sync::RwLock;

use crate::{cache::TtlCache, config::Config, near::NearClient};

mod cache;
mod config;
mod near;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::init();

    let app = Router::new()
        .route("/near/:address", post(near))
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

enum AppError {
    TooManyRequests,
    Anyhow(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Anyhow(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)).into_response()
            }
            Self::TooManyRequests => {
                (StatusCode::TOO_MANY_REQUESTS, "Too many requests").into_response()
            }
        }
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Anyhow(err.into())
    }
}
