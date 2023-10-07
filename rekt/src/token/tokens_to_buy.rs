use std::sync::Arc;

use dashmap::DashMap;
use futures::StreamExt;
use tokio::time::interval;

use super::Token;

const TOKENS_TO_BUY_FILE_PATH: &str = "tokens_to_buy.json";

#[derive(Debug, Clone)]
pub struct TokensToBuy {
    // NOTE: override hasher here, since addresses are already hashed
    tokens: DashMap<ethers::types::Address, Token>,
}

impl TokensToBuy {
    pub fn new() -> Self {
        Self {
            tokens: DashMap::new(),
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
                        println!("Added token to buy: {}", token.buy_token_address);
                        self.tokens.insert(token.get_key(), token);
                    }
                }
                Err(e) => {
                    println!("Error reading tokens to buy from file: {}", e);
                }
            }
        }
    }

    pub fn get_all(&self) -> Vec<Token> {
        self.tokens.iter().map(|v| v.clone()).collect()
    }

    pub fn has_token(&self, token_address: &ethers::types::Address) -> bool {
        self.tokens.contains_key(token_address)
    }

    pub fn get_and_remove_token(&self, token_address: ethers::types::Address) -> Option<Token> {
        self.tokens.remove(&token_address).map(|v| v.1)
    }

    async fn read_tokens_to_buy_from_file() -> Result<Vec<Token>, std::io::Error> {
        let tokens_to_buy_file = tokio::fs::read_to_string(TOKENS_TO_BUY_FILE_PATH).await?;
        let tokens_to_buy: Vec<Token> = serde_json::from_str(&tokens_to_buy_file)?;
        Ok(tokens_to_buy)
    }
}
