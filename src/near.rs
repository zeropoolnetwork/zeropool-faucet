use std::str::FromStr;

use anyhow::{anyhow, Result};
use near_crypto::{InMemorySigner, SecretKey};
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::{
    transaction::{Action, Transaction, TransferAction},
    types::{AccountId, BlockReference},
    views::FinalExecutionStatus,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NearConfig {
    pub rpc_url: String,
    pub account_id: String,
    pub secret_key: String,
    pub amount: u128,
}

pub struct NearClient {
    client: JsonRpcClient,
    signer: InMemorySigner,
}

impl NearClient {
    pub fn new(config: &NearConfig) -> Result<Self> {
        let client = JsonRpcClient::connect(&config.rpc_url);

        let account_id = AccountId::from_str(&config.account_id)?;
        let secret_key = SecretKey::from_str(&config.secret_key)?;
        let signer = InMemorySigner::from_secret_key(account_id, secret_key);

        Ok(Self { client, signer })
    }

    pub async fn transfer(&self, to: &str, amount: u128) -> Result<()> {
        let access_key_query_response = self
            .client
            .call(methods::query::RpcQueryRequest {
                block_reference: BlockReference::latest(),
                request: near_primitives::views::QueryRequest::ViewAccessKey {
                    account_id: self.signer.account_id.clone(),
                    public_key: self.signer.public_key.clone(),
                },
            })
            .await?;

        let nonce = match access_key_query_response.kind {
            QueryResponseKind::AccessKey(access_key) => access_key.nonce,
            _ => return Err(anyhow!("failed to extract current nonce")),
        };

        let transaction = Transaction {
            signer_id: self.signer.account_id.clone(),
            public_key: self.signer.public_key.clone(),
            nonce: nonce + 1,
            receiver_id: to.parse()?,
            block_hash: access_key_query_response.block_hash,
            actions: vec![Action::Transfer(TransferAction { deposit: amount })],
        };

        let request = methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
            signed_transaction: transaction.sign(&self.signer),
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
                        account_id: self.signer.account_id.clone(),
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
