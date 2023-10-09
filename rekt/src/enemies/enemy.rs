use std::str::FromStr;

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::{
    constants::{TOKEN_IN_TX_ENDS_AT, TOKEN_IN_TX_STARTS_AT},
    token::token::TokenAddress,
};

pub type EnemyAddress = ethers::types::Address;
pub type EnemyPrepareMethodSignature = ethers::types::H32;

pub static ENEMIES: Lazy<DashMap<EnemyPrepareMethodSignature, Enemy>> = Lazy::new(|| {
    let enemies = vec![
        Enemy::new_default(
            "Figa".to_string(),
            EnemyAddress::from_str("0x3dca07e16b2becd3eb76a9f9ce240b525451f887")
                .expect("Figa wallet address should be valid"),
            EnemyPrepareMethodSignature::from_str("0xbb0b896c")
                .expect("Figa prepare method signature should be valid"),
        ),
        Enemy::new_default(
            "Figa2".to_string(),
            EnemyAddress::from_str("0x3dca07e16b2becd3eb76a9f9ce240b525451f887")
                .expect("Figa2 wallet address should be valid"),
            EnemyPrepareMethodSignature::from_str("0xb11e3d9c")
                .expect("Figa prepare method signature should be valid"),
        ),
    ];

    DashMap::from_iter(
        enemies
            .into_iter()
            .map(|enemy| (enemy.prepare_method_signature, enemy)),
    )
});

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

    pub fn enemy_is_preparing_to_buy_token(data: &[u8]) -> Option<TokenAddress> {
        if data.len() < 4 {
            return None;
        }

        if ENEMIES.is_empty() {
            return None;
        }

        let prepare_method_signature = EnemyPrepareMethodSignature::from_slice(&data[..4]);

        if let Some(enemy) = ENEMIES.get(&prepare_method_signature) {
            return (enemy.extract_token)(data).ok();
        }

        None
    }
}
