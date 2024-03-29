use std::collections::HashMap;

use anyhow::Result;

const DEFAULT_RESET_INTERVAL: u64 = 60000 * 60 * 24;

#[derive(Debug, Clone)]
pub enum BackendConfig {
    Near(crate::clients::near::NearConfig),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    /// Full limit reset interval
    pub reset_interval: u64,
    pub backends: Vec<BackendConfig>,
}

impl Config {
    pub fn init() -> Result<Self> {
        let backends_raw: HashMap<String, serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string("backends.json")?)?;

        let mut backends = Vec::new();
        for (name, config) in backends_raw {
            match name.as_str() {
                "near" => backends.push(BackendConfig::Near(serde_json::from_value(config)?)),
                _ => panic!("Unknown backend: {}", name),
            }
        }

        Ok(Config {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "80".to_string())
                .parse()?,
            reset_interval: std::env::var("RESET_INTERVAL")
                .map(|s| s.parse().unwrap())
                .unwrap_or(DEFAULT_RESET_INTERVAL),
            backends,
        })
    }
}
