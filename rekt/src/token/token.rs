use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};

use crate::{
    eth::eth_message::EthMessage,
    utils::wei_gwei_converter::{
        gas_price_is_in_supported_range, gas_price_to_index, get_default_gas_price_range,
        gwei_to_wei, gwei_to_wei_with_decimals, DEFAULT_GWEI_DECIMAL_PRECISION, MIN_GAS_PRICE,
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

    #[serde(rename = "skipTest", default)]
    pub skip_protection: bool,

    #[serde(rename = "testPercent")]
    pub protection_percent: u16,

    #[serde(rename = "tokenBuyLimit", default)]
    pub max_token_buy_limit: u64,

    #[serde(rename = "enableBuyConfig")]
    pub enable_buy_config: EnableBuyConfig,

    #[serde(rename = "sellConfig", default)]
    pub sell_config: SellConfig,

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SellConfig {
    #[serde(rename = "gasPrice", default = "default_gas_price")]
    pub gas_price: u64,
    #[serde(rename = "doNotSell", default)]
    pub transfer_instead_of_selling: bool,
    #[serde(rename = "sellCount", default = "default_sell_count")]
    pub sell_count: u16,
    #[serde(rename = "firstSellPercent", default = "default_first_sell_percent")]
    pub first_sell_percent: u16,
    #[serde(rename = "percentToKeep", default)]
    pub percent_to_keep: u16,
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

    pub fn get_buy_txs(&self, gas_price_in_wei: u64) -> Option<EthMessage> {
        let gas_price_in_wei = U256::from(gas_price_in_wei);
        if !gas_price_is_in_supported_range(gas_price_in_wei) {
            color_print::cprintln!(
                "<red>Gas price is not in supported range: {}</>",
                gas_price_in_wei
            );
            return None;
        }

        self.buy_txs.as_ref().map(|txs| {
            let index = gas_price_to_index(gas_price_in_wei, DEFAULT_GWEI_DECIMAL_PRECISION);
            println!("index: {}", index);
            txs[index].clone()
        })
    }
}

impl Default for SellConfig {
    fn default() -> Self {
        Self {
            gas_price: default_gas_price(),
            sell_count: default_sell_count(),
            first_sell_percent: default_first_sell_percent(),
            transfer_instead_of_selling: false,
            percent_to_keep: 0,
        }
    }
}

fn default_gas_price() -> u64 {
    MIN_GAS_PRICE
}

fn default_sell_count() -> u16 {
    1
}

fn default_first_sell_percent() -> u16 {
    100
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
        },
        "sellConfig": {
             "sellCount": 2
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
                sell_config: SellConfig {
                    sell_count: 2,
                    ..SellConfig::default()
                },
                skip_protection: false,
                buy_txs: None,
                max_token_buy_limit: 0
            }
        );
    }
}
