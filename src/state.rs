use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::RwLock;

use crate::{
    cache::AddrCache,
    clients::{near::Token, Client},
    config::{BackendConfig, Config},
};

type ChainName = String;
type TokenAddress = String;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub chains: HashMap<ChainName, Backend>,
}

#[derive(Clone)]
pub struct Backend {
    pub client: Arc<dyn Client + Send + Sync>,
    pub caches: Arc<RwLock<HashMap<TokenAddress, AddrCache>>>,
}

impl AppState {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let interval = Duration::from_millis(config.reset_interval);

        let mut chains = HashMap::new();
        for backend in &config.backends {
            match backend {
                BackendConfig::Near(config) => {
                    let mut caches = HashMap::new();
                    for token in &config.tokens {
                        let (token, token_name) = match token {
                            Token::Near(token) => (token, "near".to_owned()),
                            Token::Ft(token) => (token, token.account_id.clone()),
                        };

                        caches.insert(
                            token_name,
                            AddrCache::new(interval, token.limit.parse().unwrap()),
                        );
                    }

                    chains.insert(
                        "near".to_string(),
                        Backend {
                            client: Arc::new(crate::clients::near::NearClient::new(config)?),
                            caches: Arc::new(RwLock::new(caches)),
                        },
                    );
                }
            }
        }

        Ok(Self {
            config: config.clone(),
            chains,
        })
    }
}
