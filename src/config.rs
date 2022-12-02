use std::time::Duration;

use serde::Deserialize;

use crate::near::NearConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub near: NearConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub interval: u64,
}

impl Config {
    pub fn init() -> Self {
        Config {
            server: envy::from_env().unwrap(),
            near: envy::prefixed("NEAR_").from_env().unwrap(),
        }
    }
}
