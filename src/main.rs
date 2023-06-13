use std::net::SocketAddr;

use anyhow::anyhow;
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
mod clients;
mod config;
mod error;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::init();

    let cors = CorsLayer::new().allow_origin(Any).allow_headers(Any);

    let app = Router::new()
        .route("/:chain/:token/:to", post(mint))
        .route("/info", get(info))
        .with_state(AppState::new(&config)?)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
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
async fn mint(
    state: State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path((chain, token, to)): Path<(String, String, String)>,
    Json(req_data): Json<FaucetReq>,
) -> Result<(), AppError> {
    let amount_n = req_data.amount.parse()?;

    let chain = state
        .chains
        .get(&chain)
        .ok_or_else(|| anyhow!("Unknown chain"))?;

    if !chain
        .caches
        .read()
        .await
        .get(&token)
        .ok_or_else(|| anyhow!("Unknown token"))?
        .can_spend(&to, addr.ip(), amount_n)
    {
        tracing::info!(
            "Address {}, {} tried to request funds over the limit ({})",
            addr,
            to,
            amount_n,
        );
        return Err(AppError::LimitExceeded);
    }

    chain.client.transfer(&to, &token, &req_data.amount).await?;

    tracing::debug!("Updating cache for {} {}", addr.ip(), to);
    chain
        .caches
        .write()
        .await
        .get_mut(&token)
        .ok_or_else(|| anyhow!("Unknown token"))?
        .spend(&to, addr.ip(), amount_n);

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
