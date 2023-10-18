use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::{
    eth::eth_message::EthMessage,
    utils::wei_gwei_converter::{
        get_default_gas_price_range, gwei_to_wei, gwei_to_wei_with_decimals,
        DEFAULT_GWEI_DECIMAL_PRECISION,
    },
    wallets::local_wallets::{
        generate_and_rlp_encode_buy_txs_for_local_wallets, update_nonces_for_local_wallets,
    },
};

pub type TokenAddress = ethers::types::Address;
pub type TxSignatureHash = ethers::types::H32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    #[serde(rename = "buyToken")]
    pub buy_token_address: TokenAddress,
    #[serde(rename = "liqToken")]
    pub liquidity_token_address: TokenAddress,

    #[serde(rename = "buyBNB")]
    pub buy_amount: f64,
    #[serde(rename = "testPercent")]
    pub protection_percent: u16,

    #[serde(rename = "enableBuyConfig")]
    pub enable_buy_config: EnableBuyConfig,

    #[serde(skip)]
    pub buy_txs: Option<Vec<EthMessage>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnableBuyConfig {
    #[serde(rename = "to")]
    pub tx_to: TokenAddress,
    #[serde(rename = "txHash")]
    pub enable_buy_tx_hash: TxSignatureHash,
}

impl Token {
    pub fn get_key(&self) -> Address {
        self.enable_buy_config.tx_to
    }

    pub async fn prepare_buy_txs_per_gas_price(&mut self) {
        update_nonces_for_local_wallets().await;
        let gas_price_range = get_default_gas_price_range();
        let mut buy_txs =
            Vec::with_capacity((gas_price_range.end() - gas_price_range.start() + 1) as usize);

        for gwei in gas_price_range {
            let txs = generate_and_rlp_encode_buy_txs_for_local_wallets(gwei_to_wei_with_decimals(
                gwei,
                DEFAULT_GWEI_DECIMAL_PRECISION,
            ))
            .await;
            buy_txs.push(EthMessage::new_tx_message(txs));
        }

        self.buy_txs = Some(buy_txs);
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn parse_token() {
        let json = r#"{
        "buyToken": "0xaE01f96CB9ce103A6A1297CC19EC0d0814Cf4c7F",
        "liqToken": "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
        "buyBNB": 100.5,
        "testPercent": 10,
        "enableBuyConfig": {
            "to": "0xCF4217DB0Ea759118d5218eFdCE88B5822859D62",
            "txHash": "0x7d315a2e"
        }
      }"#;

        let token: Token = serde_json::from_str(json).unwrap();
        assert_eq!(
            token,
            Token {
                buy_token_address: TokenAddress::from_str(
                    "0xaE01f96CB9ce103A6A1297CC19EC0d0814Cf4c7F"
                )
                .unwrap(),
                liquidity_token_address: TokenAddress::from_str(
                    "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"
                )
                .unwrap(),
                buy_amount: 100.5,
                protection_percent: 10,
                enable_buy_config: EnableBuyConfig {
                    tx_to: Address::from_str("0xCF4217DB0Ea759118d5218eFdCE88B5822859D62").unwrap(),
                    enable_buy_tx_hash: ethers::types::H32::from_str("0x7d315a2e").unwrap(),
                },
                buy_txs: None
            }
        );
    }
}
