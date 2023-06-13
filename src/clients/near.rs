use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use near_crypto::{InMemorySigner, SecretKey};
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::{
    hash::CryptoHash,
    transaction::{Action, Transaction, TransferAction},
    types::{AccountId, BlockReference},
    views::FinalExecutionStatus,
};
use serde::Deserialize;

use super::Client;

#[derive(Debug, Clone, Deserialize)]
pub struct NearConfig {
    pub rpc_url: String,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Token {
    Near(TokenConfig),
    Ft(TokenConfig),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenConfig {
    pub account_id: String,
    pub secret_key: String,
    pub limit: String,
}

struct TokenState {
    signer: InMemorySigner,
}

pub struct NearClient {
    client: JsonRpcClient,
    tokens: HashMap<String, TokenState>,
}

impl NearClient {
    pub fn new(config: &NearConfig) -> Result<Self> {
        let client = JsonRpcClient::connect(&config.rpc_url);

        let token_configs: Vec<Token> =
            serde_json::from_str(&std::fs::read_to_string("tokens.json")?)?;

        let mut tokens = HashMap::new();
        for token in &token_configs {
            let token = match token {
                Token::Near(token) => token,
                Token::Ft(token) => token,
            };

            let account_id = AccountId::from_str(&token.account_id)?;
            let secret_key = SecretKey::from_str(&token.secret_key)?;
            let signer = InMemorySigner::from_secret_key(account_id, secret_key);

            tokens.insert(token.account_id.clone(), TokenState { signer });
        }

        Ok(Self { client, tokens })
    }

    async fn get_nonce_and_hash(&self, token_state: &TokenState) -> Result<(u64, CryptoHash)> {
        let access_key_query_response = self
            .client
            .call(methods::query::RpcQueryRequest {
                block_reference: BlockReference::latest(),
                request: near_primitives::views::QueryRequest::ViewAccessKey {
                    account_id: token_state.signer.account_id.clone(),
                    public_key: token_state.signer.public_key.clone(),
                },
            })
            .await?;

        let nonce = match access_key_query_response.kind {
            QueryResponseKind::AccessKey(access_key) => access_key.nonce,
            _ => return Err(anyhow!("failed to extract current nonce")),
        };

        Ok((nonce + 1, access_key_query_response.block_hash))
    }
}

#[async_trait]
impl Client for NearClient {
    async fn transfer(&self, to: &str, token: &str, amount: &str) -> Result<()> {
        let token_state = self
            .tokens
            .get(token)
            .ok_or_else(|| anyhow!("token not found"))?;

        let amount = amount.parse::<u128>()?;

        let (nonce, block_hash) = self.get_nonce_and_hash(&token_state).await?;

        let actions = if token == "near" {
            vec![Action::Transfer(TransferAction { deposit: amount })]
        } else {
            vec![Action::FunctionCall(
                near_primitives::transaction::FunctionCallAction {
                    method_name: "ft_transfer".to_string(),
                    args: serde_json::json!({
                        "receiver_id": to,
                        "amount": amount.to_string(),
                    })
                    .to_string()
                    .into_bytes(),
                    gas: 20_000_000_000_000,
                    deposit: 1,
                },
            )]
        };

        let transaction = Transaction {
            signer_id: token_state.signer.account_id.clone(),
            public_key: token_state.signer.public_key.clone(),
            nonce: nonce + 1,
            receiver_id: to.parse()?,
            block_hash,
            actions,
        };

        let request = methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
            signed_transaction: transaction.sign(&token_state.signer),
        };

        let response = self.client.call(request).await?;
        tracing::info!(
            "Transaction hash: {}, status: {:?}",
            response.transaction.hash,
            response.status
        );

        loop {
            let res = self
                .client
                .call(methods::tx::RpcTransactionStatusRequest {
                    transaction_info: methods::tx::TransactionInfo::TransactionId {
                        hash: response.transaction.hash,
                        account_id: token_state.signer.account_id.clone(),
                    },
                })
                .await?;

            match res.status {
                FinalExecutionStatus::Failure(err) => {
                    tracing::warn!("Transaction failed: {}", &err);
                    return Err(anyhow!("{}", err));
                }
                FinalExecutionStatus::SuccessValue(_) => {
                    tracing::info!("Transaction {} succeeded", &response.transaction.hash);
                    break;
                }
                _ => continue,
            }
        }

        Ok(())
    }
}
