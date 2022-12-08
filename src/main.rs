use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::{config::Config, error::*, state::*};

mod cache;
mod config;
mod error;
mod near;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::init();

    let cors = CorsLayer::new().allow_origin(Any).allow_headers(Any);

    let app = Router::new()
        .route("/near/:to", post(near))
        .route("/info", get(info))
        .with_state(AppState::new(&config)?)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

#[derive(Deserialize)]
struct FaucetReq {
    amount: String,
}

#[axum::debug_handler]
async fn near(
    state: State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(to): Path<String>,
    Json(req_data): Json<FaucetReq>,
) -> Result<(), AppError> {
    tracing::debug!("{}", to);

    let amount = req_data.amount.parse()?;

    if !state
        .near_cache
        .read()
        .await
        .can_spend(&to, addr.ip(), amount)
    {
        tracing::info!(
            "Address {}, {} tried to request funds over the limit ({})",
            addr,
            to,
            amount,
        );
        return Err(AppError::LimitExceeded);
    }

    let amount = state.config.near.amount.parse()?;

    state.near_client.transfer(&to, amount).await?;

    tracing::debug!("Updating cache for {} {}", addr.ip(), to);
    state.near_cache.write().await.spend(to, addr.ip(), amount);

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
