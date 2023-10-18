use std::{convert::Infallible, str::FromStr};

use derive_more::Display;
use ethers::types::Address;
use warp::{reject::Reject, Filter};

use crate::{
    token::tokens_to_buy::get_token_by_address,
    wallets::local_wallets::generate_and_rlp_encode_prep_tx,
};

pub fn run_local_server() {
    tokio::task::spawn(async move {
        let prep = warp::path!("prep" / String).and_then(|token_address: String| async move {
            let token_address = match Address::from_str(&token_address) {
                Ok(t_a) => t_a,
                Err(_) => return Err(warp::reject::custom(LocalServerErr::InvalidTokenAddress)),
            };

            let token = get_token_by_address(&token_address);
            if token.is_none() {
                return Err(warp::reject::custom(LocalServerErr::TokenNotFound));
            }
            let token = token.unwrap();
            generate_and_rlp_encode_prep_tx(token).await;

            Ok(format!("Prep sent successfully: {}", token_address))
        });
        warp::serve(prep).run(([0, 0, 0, 0], 6060)).await;
    });
}

#[derive(Debug, Display)]
enum LocalServerErr {
    InvalidTokenAddress,
    TokenNotFound,
}

impl Reject for LocalServerErr {}
