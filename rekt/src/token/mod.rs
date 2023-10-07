use ethers::types::Address;
use serde::{Deserialize, Serialize};

type TxSignatureHash = ethers::types::H32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub buy_token_address: Address,
    pub liquidity_token_address: Address,

    pub buy_amount: f64,
    pub protection_percent: u16,

    pub enable_buy_config: EnableBuyConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnableBuyConfig {
    pub tx_to: Address,
    pub enable_buy_tx_hash: TxSignatureHash,
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn parse_token() {
        let json = r#"{
        "buy_token_address": "0xaE01f96CB9ce103A6A1297CC19EC0d0814Cf4c7F",
        "liquidity_token_address": "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
        "buy_amount": 100.5,
        "protection_percent": 10,
        "enable_buy_config": {
            "tx_to": "0xCF4217DB0Ea759118d5218eFdCE88B5822859D62",
            "enable_buy_tx_hash": "0x7d315a2e"
        }
      }"#;

        let token: Token = serde_json::from_str(json).unwrap();
        assert_eq!(
            token,
            Token {
                buy_token_address: Address::from_str("0xaE01f96CB9ce103A6A1297CC19EC0d0814Cf4c7F")
                    .unwrap(),
                liquidity_token_address: Address::from_str(
                    "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"
                )
                .unwrap(),
                buy_amount: 100.5,
                protection_percent: 10,
                enable_buy_config: EnableBuyConfig {
                    tx_to: Address::from_str("0xCF4217DB0Ea759118d5218eFdCE88B5822859D62").unwrap(),
                    enable_buy_tx_hash: ethers::types::H32::from_str("0x7d315a2e").unwrap(),
                }
            }
        );
    }
}
