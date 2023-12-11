use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};

use crate::{
    constants::{TX_ARG_LEN, TX_SIGNATURE_LEN},
    eth::eth_message::EthMessage,
    utils::wei_gwei_converter::{
        gas_price_is_in_supported_precision, gas_price_is_in_supported_range, gas_price_to_index,
        get_default_gas_price_range, gwei_to_wei_with_decimals, DEFAULT_GWEI_DECIMAL_PRECISION,
        MIN_GAS_PRICE,
    },
    wallets::local_wallets::{
        generate_and_rlp_encode_buy_txs_for_local_wallets, update_nonces_for_local_wallets,
    },
};

pub type TokenAddress = ethers::types::Address;
pub type TxSignatureHash = ethers::types::H32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    #[serde(rename = "v", default)]
    pub version: u8,
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

    /// This means we won't do manual prep so prep tx
    /// needs to be sent alongside  buy txs
    #[serde(rename = "doPrep", default)]
    pub prep_in_flight: bool,

    #[serde(rename = "isPcs", default)]
    pub liq_will_be_added_via_pcs: bool,

    #[serde(default)]
    pub from: Option<FromConfig>,

    #[serde(skip)]
    pub buy_txs: Option<Vec<EthMessage>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnableBuyConfig {
    #[serde(rename = "to")]
    pub tx_to: TokenAddress,
    #[serde(rename = "txHash")]
    pub enable_buy_tx_hash: TxSignatureHash,
    /// TX to enable buy has argument which controls if buy is enabled
    /// eg. enable_trade(is_enabled: bool)
    /// but this argument can be in any place eg.
    /// enable_trade(some_other_arg: u8, is_enabled: bool)
    #[serde(rename = "tradeStatusArgPos", default)]
    pub trade_status_arg_position: usize,

    /// TX to enable buy has argument which controls if buy is enabled
    /// but argument can be of any value, eg.
    /// enable_trade(some_other_arg: u8, is_disabled: bool)
    /// in this case we want to buy if is_disabled is 0
    /// it's important that we know upfront which value will be used to enable buy
    #[serde(rename = "tradeStatusArgValue", default = "default_trade_status_value")]
    pub trade_status_arg_value: u8,

    /// TX to enable buy has argument which controls if buy is enabled
    /// but argument can be of any value, eg. and we don't know which value will be used to enable buy
    /// but we know itn won't be 0, eg.
    /// startTrade(address[] addresses,uint256 r)
    /// in this concrete scenario (specified in token contract)
    /// we want to buy if addresses.len() > 0
    /// the way arrays are encoded in TX data is that before array values
    /// we have array length, so we can check if array length is > 0
    #[serde(rename = "tradeStatusArgValueNoZero", default)]
    pub trade_status_arg_value_any_bigger_than_0: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FromConfig {
    #[serde(rename = "minNonce", default)]
    pub min_nonce: u64,
    #[serde(rename = "maxNonce", default)]
    pub max_nonce: u64,
}

impl Token {
    pub fn get_key(&self) -> Address {
        self.enable_buy_config.tx_to
    }

    pub async fn prepare_buy_txs_for_gas_price(&mut self, gas_price_in_wei: u64) -> EthMessage {
        let txs =
            generate_and_rlp_encode_buy_txs_for_local_wallets(&self, U256::from(gas_price_in_wei))
                .await;
        EthMessage::new_compressed_tx_message(txs)
    }

    pub async fn prepare_buy_txs_for_gas_price_range(&mut self) {
        update_nonces_for_local_wallets().await;
        let gas_price_range = get_default_gas_price_range();
        let mut buy_txs =
            Vec::with_capacity((gas_price_range.end() - gas_price_range.start() + 1) as usize);

        for gwei in gas_price_range {
            let wei = gwei_to_wei_with_decimals(gwei, DEFAULT_GWEI_DECIMAL_PRECISION);
            let txs = generate_and_rlp_encode_buy_txs_for_local_wallets(&self, wei).await;

            buy_txs.push(EthMessage::new_compressed_tx_message(txs));
        }

        self.buy_txs = Some(buy_txs);
    }

    pub fn get_buy_txs(&mut self, gas_price_in_wei: u64) -> Option<EthMessage> {
        let gas_price_in_wei = U256::from(gas_price_in_wei);
        if !gas_price_is_in_supported_range(gas_price_in_wei) {
            color_print::cprintln!(
                "<red>Gas price is not in supported range: {}</>",
                gas_price_in_wei
            );
            return None;
        }

        if !gas_price_is_in_supported_precision(gas_price_in_wei, DEFAULT_GWEI_DECIMAL_PRECISION) {
            return None;
        }

        self.buy_txs.as_mut().map(|txs| {
            let index = gas_price_to_index(gas_price_in_wei, DEFAULT_GWEI_DECIMAL_PRECISION);
            txs.swap_remove(index)
        })
    }

    #[inline(always)]
    pub fn trade_status_is_enable(&self, tx_data: &[u8]) -> bool {
        if self.enable_buy_config.trade_status_arg_position == 0 {
            return true;
        }

        let arg_position =
            TX_SIGNATURE_LEN + TX_ARG_LEN * self.enable_buy_config.trade_status_arg_position;
        if tx_data.len() < arg_position {
            return false;
        }

        if self
            .enable_buy_config
            .trade_status_arg_value_any_bigger_than_0
        {
            return tx_data[arg_position - 1] > 0;
        }

        tx_data[arg_position - 1] == self.enable_buy_config.trade_status_arg_value
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

fn default_trade_status_value() -> u8 {
    1
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
                version: 0,
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
                    trade_status_arg_position: 0,
                    trade_status_arg_value: 1,
                    trade_status_arg_value_any_bigger_than_0: false,
                },
                sell_config: SellConfig {
                    sell_count: 2,
                    ..SellConfig::default()
                },
                skip_protection: false,
                buy_txs: None,
                max_token_buy_limit: 0,
                prep_in_flight: false,
                from: None,
                liq_will_be_added_via_pcs: false
            }
        );
    }

    #[test]
    fn enable_buy_tx_with_status_arg() {
        let token = Token {
            version: 0,
            buy_token_address: TokenAddress::from_str("0xaE01f96CB9ce103A6A1297CC19EC0d0814Cf4c7F")
                .unwrap(),
            liquidity_token_address: TokenAddress::from_str(
                "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
            )
            .unwrap(),
            buy_amount: 100.5,
            protection_percent: 10,
            enable_buy_config: EnableBuyConfig {
                tx_to: Address::from_str("0xCF4217DB0Ea759118d5218eFdCE88B5822859D62").unwrap(),
                enable_buy_tx_hash: ethers::types::H32::from_str("0x7d315a2e").unwrap(),
                trade_status_arg_position: 2,
                trade_status_arg_value: 1,
                trade_status_arg_value_any_bigger_than_0: false,
            },
            sell_config: SellConfig {
                sell_count: 2,
                ..SellConfig::default()
            },
            skip_protection: false,
            buy_txs: None,
            max_token_buy_limit: 0,
            from: None,
            prep_in_flight: false,
            liq_will_be_added_via_pcs: false,
        };

        let tx_data = hex::decode("7d315a2e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001");

        assert_eq!(token.trade_status_is_enable(&tx_data.unwrap()), true);

        let tx_data = hex::decode("7d315a2e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(token.trade_status_is_enable(&tx_data.unwrap()), false);

        let token = Token {
            enable_buy_config: EnableBuyConfig {
                tx_to: Address::from_str("0xCF4217DB0Ea759118d5218eFdCE88B5822859D62").unwrap(),
                enable_buy_tx_hash: ethers::types::H32::from_str("0x7d315a2e").unwrap(),
                trade_status_arg_position: 1,
                trade_status_arg_value: 1,
                trade_status_arg_value_any_bigger_than_0: false,
            },
            ..token
        };
        let tx_data = hex::decode("177a634e00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002");
        assert_eq!(token.trade_status_is_enable(&tx_data.unwrap()), true);

        let token = Token {
            enable_buy_config: EnableBuyConfig {
                tx_to: Address::from_str("0xCF4217DB0Ea759118d5218eFdCE88B5822859D62").unwrap(),
                enable_buy_tx_hash: ethers::types::H32::from_str("0x7d315a2e").unwrap(),
                trade_status_arg_position: 2,
                trade_status_arg_value: 4,
                trade_status_arg_value_any_bigger_than_0: false,
            },
            ..token
        };
        let tx_data = hex::decode("7d315a2e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004");
        assert_eq!(token.trade_status_is_enable(&tx_data.unwrap()), true);

        let token = Token {
            enable_buy_config: EnableBuyConfig {
                tx_to: Address::from_str("0xCF4217DB0Ea759118d5218eFdCE88B5822859D62").unwrap(),
                enable_buy_tx_hash: ethers::types::H32::from_str("0x7d315a2e").unwrap(),
                trade_status_arg_position: 2,
                trade_status_arg_value: 4,
                trade_status_arg_value_any_bigger_than_0: true,
            },
            ..token
        };
        let tx_data = hex::decode("7d315a2e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003");
        assert_eq!(token.trade_status_is_enable(&tx_data.unwrap()), true);
    }
}
