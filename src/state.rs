use std::{sync::Arc, time::Duration};

use near_primitives::types::Balance as NearBalance;
use tokio::sync::RwLock;

use crate::{cache::AddrCache, config::Config, near::NearClient};

pub type NearCache = AddrCache<NearBalance>;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub near_client: Arc<NearClient>,
    pub near_cache: Arc<RwLock<NearCache>>,
}

impl AppState {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let interval = Duration::from_millis(config.server.interval);
        Ok(Self {
            config: config.clone(),
            near_cache: Arc::new(RwLock::new(AddrCache::new(
                interval,
                config.near.amount.parse()?,
            ))),
            near_client: Arc::new(NearClient::new(&config.near)?),
        })
    }
}
