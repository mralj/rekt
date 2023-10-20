use std::str::FromStr;

use ethers::{
    abi::Abi,
    contract::Contract,
    providers::{Http, Provider},
    types::{Address, Bytes, U256},
};
use num_traits::Pow;
use once_cell::sync::Lazy;

use crate::public_nodes::nodes::PUBLIC_NODE_URLS;
use crate::token::token::Token;

pub const BUY_TX_METHOD: &str = "cure";
pub const CAESAR_BOT_ADDRESS: &str = "0x92dA9c224b39Da0a03ede8Fb85C0F7798cfF0923";

static CAESAR_BOT: Lazy<Contract<Provider<Http>>> = Lazy::new(|| get_caesar_bot());

pub fn encode_buy_method() -> Bytes {
    let buy_tx = CAESAR_BOT
        .encode(BUY_TX_METHOD, ())
        .expect("Failed to encode buy tx");

    buy_tx
}

pub fn encode_sell_method() -> Bytes {
    let sell_tx = CAESAR_BOT
        .encode("sell", ())
        .expect("Failed to encode sell tx");

    sell_tx
}

pub fn encode_prep_method(token: &Token) -> Bytes {
    let prep_tx = CAESAR_BOT
        .encode(
            "prep",
            (
                token.liquidity_token_address,
                token.buy_token_address,
                U256::from((token.buy_amount * 10f64.pow(18)) as u64),
                token.skip_protection,
                token.protection_percent,
                U256::from(token.sell_config.sell_count),
                U256::from(token.sell_config.first_sell_percent),
                U256::from(token.sell_config.percent_to_keep),
                U256::from(token.max_token_buy_limit),
            ),
        )
        .expect("Failed to encode prep tx");

    prep_tx
}

fn get_caesar_bot() -> Contract<Provider<Http>> {
    let bot_address = Address::from_str(CAESAR_BOT_ADDRESS).expect("Invalid bot address");
    //NOTE: this looks like a shitty solution (since we are just using the first node from the list
    //and we don't even know if the node works or not)
    //but this is ok , because we are never using the node for anything
    //we need here to make ethers crate happy
    let client = Provider::<Http>::try_from(PUBLIC_NODE_URLS[0]).expect("Failed to create client");
    let bot_contract = Contract::new(bot_address, get_abi(), client.into());

    bot_contract
}

fn get_abi() -> Abi {
    let abi: Abi = serde_json::from_str(
        r#"[
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "preparer",
          "type": "address"
        },
        {
          "internalType": "address",
          "name": "seller",
          "type": "address"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "constructor"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        },
        {
          "indexed": true,
          "internalType": "bytes32",
          "name": "previousAdminRole",
          "type": "bytes32"
        },
        {
          "indexed": true,
          "internalType": "bytes32",
          "name": "newAdminRole",
          "type": "bytes32"
        }
      ],
      "name": "RoleAdminChanged",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "account",
          "type": "address"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "sender",
          "type": "address"
        }
      ],
      "name": "RoleGranted",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "account",
          "type": "address"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "sender",
          "type": "address"
        }
      ],
      "name": "RoleRevoked",
      "type": "event"
    },
    {
      "stateMutability": "payable",
      "type": "fallback"
    },
    {
      "inputs": [],
      "name": "DEFAULT_ADMIN_ROLE",
      "outputs": [
        {
          "internalType": "bytes32",
          "name": "",
          "type": "bytes32"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "PREPARE_ROLE",
      "outputs": [
        {
          "internalType": "bytes32",
          "name": "",
          "type": "bytes32"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "SELL_ROLE",
      "outputs": [
        {
          "internalType": "bytes32",
          "name": "",
          "type": "bytes32"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "balance",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "cure",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        }
      ],
      "name": "getRoleAdmin",
      "outputs": [
        {
          "internalType": "bytes32",
          "name": "",
          "type": "bytes32"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        },
        {
          "internalType": "address",
          "name": "account",
          "type": "address"
        }
      ],
      "name": "grantRole",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        },
        {
          "internalType": "address",
          "name": "account",
          "type": "address"
        }
      ],
      "name": "hasRole",
      "outputs": [
        {
          "internalType": "bool",
          "name": "",
          "type": "bool"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "token",
          "type": "address"
        }
      ],
      "name": "liqAlreadyAdded",
      "outputs": [
        {
          "internalType": "bool",
          "name": "",
          "type": "bool"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "liquidityToken",
          "type": "address"
        },
        {
          "internalType": "address",
          "name": "token",
          "type": "address"
        },
        {
          "internalType": "uint256",
          "name": "buyAmount",
          "type": "uint256"
        },
        {
          "internalType": "bool",
          "name": "skipTest",
          "type": "bool"
        },
        {
          "internalType": "uint256",
          "name": "testThreshold",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "sellCount",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "firstSellPercent",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "percentOfTokensToKeep",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "buyLimit",
          "type": "uint256"
        }
      ],
      "name": "prep",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        },
        {
          "internalType": "address",
          "name": "account",
          "type": "address"
        }
      ],
      "name": "renounceRole",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "role",
          "type": "bytes32"
        },
        {
          "internalType": "address",
          "name": "account",
          "type": "address"
        }
      ],
      "name": "revokeRole",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "sell",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "token",
          "type": "address"
        }
      ],
      "name": "sellAll",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes4",
          "name": "interfaceId",
          "type": "bytes4"
        }
      ],
      "name": "supportsInterface",
      "outputs": [
        {
          "internalType": "bool",
          "name": "",
          "type": "bool"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "withdraw",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "token",
          "type": "address"
        },
        {
          "internalType": "address",
          "name": "withdrawTo",
          "type": "address"
        }
      ],
      "name": "withdrawToken",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "stateMutability": "payable",
      "type": "receive"
    }
  ]"#,
    )
    .expect("Invalid abi");

    abi
}
