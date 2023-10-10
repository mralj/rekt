use std::str::FromStr;

use dashmap::DashMap;
use once_cell::sync::Lazy;

use super::enemy::{Enemy, EnemyAddress, EnemyPrepareMethodSignature};

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
        Enemy {
            name: "Curcinelli".to_string(),
            wallet_address: EnemyAddress::from_str("0x5610DA09A54B51A841E2dAddc0a2A74C6d37E0b3")
                .expect("Curcinelli wallet address should be valid"),
            prepare_method_signature: EnemyPrepareMethodSignature::from_str("0x917c923b")
                .expect("Curcinelli prepare method signature should be valid"),
            extract_token: Enemy::extract_token_for_curcinelli_bot,
        },
    ];

    DashMap::from_iter(
        enemies
            .into_iter()
            .map(|enemy| (enemy.prepare_method_signature, enemy)),
    )
});
