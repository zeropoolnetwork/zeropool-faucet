use anyhow::Result;
use async_trait::async_trait;

pub mod near;

#[async_trait]
pub trait Client {
    async fn transfer(&self, to: &str, token: &str, amount: &str) -> Result<()>;
}
