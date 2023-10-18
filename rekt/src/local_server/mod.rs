use std::str::FromStr;

use ethers::types::Address;
use warp::Filter;

use crate::token::tokens_to_buy::get_token_by_address;

pub fn run_local_server() {
    tokio::task::spawn(async move {
        let prep = warp::path!("prep" / String).map(|token_address: String| {
            let token_address = match Address::from_str(&token_address) {
                Ok(t_a) => t_a,
                Err(e) => return format!("Invalid token address, {e}"),
            };

            let token = get_token_by_address(&token_address);
            if token.is_none() {
                return format!("Token not found: {}", token_address);
            }
            let token = token.unwrap();

            format!("Prep sent successfully: {}", token_address)
        });
        warp::serve(prep).run(([0, 0, 0, 0], 6060)).await;
    });
}
