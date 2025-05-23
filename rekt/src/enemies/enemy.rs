use crate::{
    constants::{TOKEN_IN_TX_ENDS_AT, TOKEN_IN_TX_STARTS_AT, TX_SIGNATURE_LEN},
    token::token::TokenAddress,
};

use super::enemies_list::ENEMIES;

pub type EnemyAddress = ethers::types::Address;
pub type EnemyPrepareMethodSignature = ethers::types::H32;

pub struct Enemy {
    pub name: String,
    pub wallet_address: EnemyAddress,
    pub prepare_method_signature: EnemyPrepareMethodSignature,
    pub extract_token: fn(data: &[u8]) -> anyhow::Result<TokenAddress>,
}

impl Default for Enemy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            wallet_address: EnemyAddress::zero(),
            prepare_method_signature: EnemyPrepareMethodSignature::zero(),
            extract_token: Enemy::default_extract_token,
        }
    }
}

impl Enemy {
    pub fn new_default(
        name: String,
        wallet_address: EnemyAddress,
        prepare_method_signature: EnemyPrepareMethodSignature,
    ) -> Self {
        Self {
            name,
            wallet_address,
            prepare_method_signature,
            ..Default::default()
        }
    }
    pub fn default_extract_token(data: &[u8]) -> anyhow::Result<TokenAddress> {
        if data.len() < TOKEN_IN_TX_ENDS_AT {
            return Err(anyhow::anyhow!("Data length is too short"));
        }

        Ok(TokenAddress::from_slice(
            &data[TOKEN_IN_TX_STARTS_AT..TOKEN_IN_TX_ENDS_AT],
        ))
    }

    pub fn enemy_is_preparing_to_buy_token(data: &[u8]) -> Option<(String, TokenAddress)> {
        if data.len() < TX_SIGNATURE_LEN {
            return None;
        }

        if let Some(enemy) = ENEMIES
            .iter()
            .find(|enemy| enemy.prepare_method_signature.as_ref() == &data[..TX_SIGNATURE_LEN])
        {
            if let Ok(token) = (enemy.extract_token)(data) {
                return Some((enemy.name.clone(), token));
            }
        }

        None
    }
}
