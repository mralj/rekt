use std::str::FromStr;

use ethers::{
    prelude::k256::ecdsa::SigningKey,
    signers::{Signer, Wallet, WalletError},
    types::{transaction::eip2718::TypedTransaction, Address, Signature, U256},
};

use crate::public_nodes::nodes::get_nonce;

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

    pub async fn update_nonce(&mut self) {
        // NOTE: we update nocne only if we were able to get the value
        // this protects us from the following scenario:
        // we already have nonce (it's eg. Some(16))
        // we try to update it, but there is some error
        // we don't want to set nonce to None in this case
        if let Some(n) = get_nonce(self).await {
            self.nonce = Some(n);
        }
    }

    pub fn sign_tx(&self, tx: &TypedTransaction) -> Result<Signature, WalletError> {
        self.wallet.sign_transaction_sync(tx)
    }
}

impl FromStr for WalletWithNonce {
    type Err = WalletError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(Wallet::from_str(s)?))
    }
}
