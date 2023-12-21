use std::str::FromStr;

use ethers::{
    prelude::k256::ecdsa::SigningKey,
    signers::{Signer, Wallet, WalletError},
    types::{
        transaction::eip2718::TypedTransaction, Address, Bytes, Signature, TransactionRequest, U256,
    },
};

use crate::{
    contracts::caesar_bot::{
        encode_buy_method, encode_prep_method, encode_sell_method, CAESAR_BOT_ADDRESS,
    },
    public_nodes::nodes::get_nonce,
    token::token::Token,
};

pub type WeiGasPrice = U256;
const DEFAULT_MAX_GAS_LIMIT: usize = 4_000_000;

pub struct WalletWithNonce {
    wallet: Wallet<SigningKey>,
    nonce: Option<U256>,
}

impl WalletWithNonce {
    pub fn new(wallet: Wallet<SigningKey>) -> Self {
        Self {
            wallet,
            nonce: None,
        }
    }

    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    pub fn nonce(&self) -> Option<U256> {
        self.nonce
    }

    pub fn update_nonce_locally(&mut self) -> U256 {
        match self.nonce() {
            Some(n) => {
                self.nonce = Some(n + 1);
                n + 1
            }
            None => {
                self.nonce = Some(U256::zero());
                U256::zero()
            }
        }
    }

    pub async fn update_nonce(&mut self) -> Option<U256> {
        // NOTE: we update nocne only if we were able to get the value
        // this protects us from the following scenario:
        // we already have nonce (it's eg. Some(16))
        // we try to update it, but there is some error
        // we don't want to set nonce to None in this case
        if let Some(n) = get_nonce(self).await {
            self.nonce = Some(n);
            return Some(n);
        }
        None
    }

    pub async fn generate_and_sign_buy_tx(
        &mut self,
        gas_price: WeiGasPrice,
    ) -> Result<Bytes, WalletError> {
        let data = encode_buy_method();
        let tx = self.generate_tx_to_bot(data, gas_price).await;
        let signature = self.sign_tx(&tx)?;

        Ok(tx.rlp_signed(&signature))
    }

    pub async fn generate_and_sign_sell_tx(
        &mut self,
        gas_price: WeiGasPrice,
    ) -> Result<Bytes, WalletError> {
        let data = encode_sell_method();
        let tx = self.generate_tx_to_bot(data, gas_price).await;
        let signature = self.sign_tx(&tx)?;

        Ok(tx.rlp_signed(&signature))
    }

    pub async fn generate_and_sign_prep_tx(
        &mut self,
        token: &Token,
        gas_price: WeiGasPrice,
    ) -> Result<Bytes, WalletError> {
        let data = encode_prep_method(token);
        let tx = self.generate_tx_to_bot(data, gas_price).await;
        let signature = self.sign_tx(&tx)?;

        Ok(tx.rlp_signed(&signature))
    }

    async fn generate_tx_to_bot(&mut self, data: Bytes, gas_price: U256) -> TypedTransaction {
        if self.nonce.is_none() {
            self.update_nonce().await;
        }

        let tx = TransactionRequest {
            from: Some(self.address()),
            to: Some(ethers::types::NameOrAddress::Address(
                Address::from_str(CAESAR_BOT_ADDRESS).expect("Invalid bot address"),
            )),
            gas: Some(U256::from(DEFAULT_MAX_GAS_LIMIT)),
            gas_price: Some(gas_price),
            data: Some(data),
            nonce: self.nonce,
            chain_id: Some(ethers::types::U64::from(56)),
            ..TransactionRequest::default()
        };

        TypedTransaction::Legacy(tx)
    }

    pub async fn generate_mev_tx(&mut self, gas_price: U256) -> Result<Bytes, WalletError> {
        if self.nonce.is_none() {
            self.update_nonce().await;
        }

        let tx = TransactionRequest {
            from: Some(self.address()),
            to: Some(ethers::types::NameOrAddress::Address(self.address())),
            gas: Some(U256::from(22_000)),
            gas_price: Some(gas_price),
            data: None,
            nonce: self.nonce,
            chain_id: Some(ethers::types::U64::from(56)),
            ..TransactionRequest::default()
        };

        let tx = TypedTransaction::Legacy(tx);
        let signature = self.sign_tx(&tx)?;

        Ok(tx.rlp_signed(&signature))
    }

    fn sign_tx(&self, tx: &TypedTransaction) -> Result<Signature, WalletError> {
        self.wallet.sign_transaction_sync(tx)
    }
}

impl FromStr for WalletWithNonce {
    type Err = WalletError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(Wallet::from_str(s)?))
    }
}
