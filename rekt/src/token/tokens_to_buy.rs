use dashmap::DashSet;
use futures::StreamExt;
use once_cell::sync::Lazy;
use tokio::time::interval;

use super::token::{Token, TokenAddress};

const TOKENS_TO_BUY_FILE_PATH: &str = "tokens_to_buy.json";

pub static mut TOKENS_TO_BUY: Vec<Token> = Vec::new();
pub static BOUGHT_TOKENS: Lazy<DashSet<TokenAddress>> = Lazy::new(|| DashSet::new());

pub fn import_tokens_to_buy() {
    unsafe {
        TOKENS_TO_BUY.reserve(10);
    }
    tokio::task::spawn(async move {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(20),
        ));

        while let Some(_) = stream.next().await {
            match read_tokens_to_buy_from_file().await {
                Ok(tokens) => {
                    for token in tokens {
                        let buy_token_address = token.buy_token_address;
                        if BOUGHT_TOKENS.contains(&buy_token_address) {
                            continue;
                        }
                        unsafe {
                            if TOKENS_TO_BUY
                                .iter()
                                .any(|v| v.buy_token_address == buy_token_address)
                            {
                                continue;
                            }
                            TOKENS_TO_BUY.push(token);
                        }
                        println!("Added token to buy: {}", buy_token_address);
                    }
                }
                Err(e) => {
                    println!("Error reading tokens to buy from file: {}", e);
                }
            }
        }
    });
}

pub fn there_are_no_tokens_to_buy() -> bool {
    unsafe { TOKENS_TO_BUY.is_empty() }
}

pub fn mark_token_as_bought(buy_token_address: TokenAddress) {
    BOUGHT_TOKENS.insert(buy_token_address);
    unsafe {
        if let Some(index) = TOKENS_TO_BUY
            .iter()
            .position(|v| v.buy_token_address == buy_token_address)
        {
            TOKENS_TO_BUY.remove(index);
        }
    }
}

pub fn get_token(address: &TokenAddress) -> Option<&Token> {
    unsafe {
        TOKENS_TO_BUY
            .iter()
            .find(|v| &v.enable_buy_config.tx_to == address)
    }
}

async fn read_tokens_to_buy_from_file() -> Result<Vec<Token>, std::io::Error> {
    let tokens_to_buy_file = tokio::fs::read_to_string(TOKENS_TO_BUY_FILE_PATH).await?;
    let tokens_to_buy: Vec<Token> = serde_json::from_str(&tokens_to_buy_file)?;
    Ok(tokens_to_buy)
}

// #[derive(Debug, Clone)]
// pub struct TokensToBuy {
//     // NOTE: override hasher here, since addresses are already hashed
//     tokens: RefCell<Vec<Token>>,
//     bought_tokens: DashSet<ethers::types::Address>,
// }
//
// unsafe impl Sync for TokensToBuy {}
//
// impl TokensToBuy {
//     pub fn new() -> Self {
//         Self {
//             tokens: RefCell::new(Vec::new()),
//             bought_tokens: DashSet::new(),
//         }
//     }
//
//     pub fn start(self: Arc<Self>) {
//         tokio::task::spawn(async move {
//             self.refresh_tokens_to_buy().await;
//         });
//     }
//
//
//     pub fn is_empty(&self) -> bool {
//         self.tokens.try_borrow().is_empty()
//     }
//
//     pub fn get_all(&self) -> Vec<Token> {
//         self.tokens.iter().map(|v| v.clone()).collect()
//     }
//
//     pub fn get(&self, token_address: &TokenAddress) -> Option<&Token> {
//         self.tokens
//             .iter()
//             .find(|v| &v.buy_token_address == token_address)
//     }
//
//     pub fn mark_token_as_bought(&self, buy_token_address: &TokenAddress) {
//         self.bought_tokens.insert(*buy_token_address);
//     }
//
//     pub fn token_already_bought(&self, token_address: &TokenAddress) -> bool {
//         self.bought_tokens.contains(token_address)
//     }
//
//     pub fn remove(&mut self, token_address: &TokenAddress) -> Option<Token> {
//         let index = self
//             .tokens
//             .iter()
//             .position(|v| &v.buy_token_address == token_address)?;
//
//         Some(self.tokens.remove(index))
//     }
//

// }
