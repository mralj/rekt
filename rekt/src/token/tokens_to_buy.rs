use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use futures::StreamExt;
use tokio::time::interval;

use super::token::{Token, TokenAddress};

const TOKENS_TO_BUY_FILE_PATH: &str = "tokens_to_buy.json";

#[derive(Debug, Clone)]
pub struct TokensToBuy {
    // NOTE: override hasher here, since addresses are already hashed
    tokens: DashMap<ethers::types::Address, Token>,
    bought_tokens: DashSet<ethers::types::Address>,
}

impl TokensToBuy {
    pub fn new() -> Self {
        Self {
            tokens: DashMap::new(),
            bought_tokens: DashSet::new(),
        }
    }

    pub fn start(self: Arc<Self>) {
        tokio::task::spawn(async move {
            self.refresh_tokens_to_buy().await;
        });
    }

    pub async fn refresh_tokens_to_buy(&self) {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(20),
        ));

        while let Some(_) = stream.next().await {
            match TokensToBuy::read_tokens_to_buy_from_file().await {
                Ok(tokens) => {
                    for token in tokens {
                        let buy_token_address = token.buy_token_address;
                        if self.token_already_bought(&buy_token_address) {
                            continue;
                        }
                        if self.tokens.insert(token.get_key(), token).is_none() {
                            println!("Added token to buy: {}", buy_token_address);
                        }
                    }
                }
                Err(e) => {
                    println!("Error reading tokens to buy from file: {}", e);
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn get_all(&self) -> Vec<Token> {
        self.tokens.iter().map(|v| v.clone()).collect()
    }

    pub fn get(
        &self,
        token_address: &TokenAddress,
    ) -> Option<dashmap::mapref::one::Ref<'_, TokenAddress, Token>> {
        self.tokens.get(token_address)
    }

    pub fn mark_token_as_bought(&self, buy_token_address: &TokenAddress) {
        self.bought_tokens.insert(*buy_token_address);
    }

    pub fn token_already_bought(&self, token_address: &TokenAddress) -> bool {
        self.bought_tokens.contains(token_address)
    }

    pub fn remove(&self, token_address: &TokenAddress) -> Option<Token> {
        self.tokens.remove(token_address).map(|v| v.1)
    }

    async fn read_tokens_to_buy_from_file() -> Result<Vec<Token>, std::io::Error> {
        let tokens_to_buy_file = tokio::fs::read_to_string(TOKENS_TO_BUY_FILE_PATH).await?;
        let tokens_to_buy: Vec<Token> = serde_json::from_str(&tokens_to_buy_file)?;
        Ok(tokens_to_buy)
    }
}
