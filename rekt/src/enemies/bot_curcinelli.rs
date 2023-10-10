use crate::{
    constants::{TX_ARG_LEN, TX_ARG_LEN_OF_ADDRESS, TX_SIGNATURE_LEN},
    token::token::TokenAddress,
};

use super::enemy::Enemy;

impl Enemy {
    // the way Curcinelli does prep is that he send TX in which last argument
    // is array of token addresses (which probably represents buying route)
    // the way arrays are represented in TX data is that first 32 bytes is
    // length of array and then array elements follow
    //
    // The extraction will work as follows:
    // chunk tx data into 32 byte chunks  (each representing one argument of tx)
    // Since the argument of interest is the last one, we iterate in reverse
    // Find first argument which can be converted to number
    // The token of interest is the one before that
    // (because Curcinelli sends  the buying token as first element of array)
    // and it's "one before" because we are iterating in reverse
    pub fn extract_token_for_curcinelli_bot(data: &[u8]) -> anyhow::Result<TokenAddress> {
        if data.len() < TX_SIGNATURE_LEN + TX_ARG_LEN {
            return Err(anyhow::anyhow!("Data length is too short"));
        }

        let tx_args: Vec<_> = data[TX_SIGNATURE_LEN..].chunks(TX_ARG_LEN).collect();

        let position_of_last_number = tx_args.iter().rev().position(|tx_arg| {
            tx_arg.len() == TX_ARG_LEN && tx_arg[..TX_ARG_LEN - 1].iter().all(|&x| x == 0)
        });

        match position_of_last_number {
            Some(index) => {
                // index is reversed, so we need to reverse it back
                // and go one backwards to get the buy token address
                let target_arg = &tx_args[tx_args.len() - index - 2];
                if target_arg.len() != TX_ARG_LEN {
                    return Err(anyhow::anyhow!("Invalid address arg length"));
                }
                let token_address =
                    TokenAddress::from_slice(&target_arg[TX_ARG_LEN - TX_ARG_LEN_OF_ADDRESS..]);
                Ok(token_address)
            }
            None => Err(anyhow::anyhow!("Could not find any numbers in tx data")),
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn curcinelli_prep_3_array_token() {
        let data = "0721806300000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000024000000000000000000000000000000000000000000000000000000000000002c000000000000000000000000000000000000000000000021e19e0c9bab23fffff00000000000000000000000000000000000000000000021e19e0c9bab23fffff0000000000000000000000000000000000000000000000000000000000000032000000000000000000000000000000000000000000000000000110d9316ec00000000000000000000000000000000000000000000000000000000000000000550000000000000000000000000000000000000000000000000000000000000046000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000690836c0af5f56000000000000000000000000000008555712347c5a0198abc2f855759083d86f631620000000000000000000000000000000000000000000000000000000000000003000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c00000000000000000000000055d398326f99059ff775485246999027b319795500000000000000000000000035359f21abdf0f2b6ae01bfb96814738b515111e000000000000000000000000000000000000000000000000000000000000000300000000000000000000000035359f21abdf0f2b6ae01bfb96814738b515111e00000000000000000000000055d398326f99059ff775485246999027b3197955000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c";

        let expected_token =
            TokenAddress::from_str("0x35359f21abdf0f2b6ae01bfb96814738b515111e").unwrap();

        let mut data = hex::decode(data).unwrap();
        let token_address = Enemy::extract_token_for_curcinelli_bot(&mut data).unwrap();
        assert_eq!(token_address, expected_token,);
    }

    #[test]
    fn curcinelli_prep_2_array_token() {
        let data = "0721806300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024000000000000000000000000000000000000000000000000000000000000002a00000000000000000000000000000000000000000002888275295397bd1000000000000000000000000000000000000000000000000295be96e640669720000000000000000000000000000000000000000000000000000000000000000000032000000000000000000000000000000000000000000000000000110d9316ec0000000000000000000000000000000000000000000000000000000000000000055000000000000000000000000000000000000000000000000000000000000004600000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000960db77681e940000000000000000000000000000ac93b323cdf11b4a321e4bc933f1cd7a436e79ef0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c0000000000000000000000004c3d5af5c43dbecee525327e93d51eb4d6ddabec00000000000000000000000000000000000000000000000000000000000000020000000000000000000000004c3d5af5c43dbecee525327e93d51eb4d6ddabec000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c";

        let expected_token =
            TokenAddress::from_str("0x4c3d5af5c43dbecee525327e93d51eb4d6ddabec").unwrap();

        let mut data = hex::decode(data).unwrap();
        let token_address = Enemy::extract_token_for_curcinelli_bot(&mut data).unwrap();
        assert_eq!(token_address, expected_token,);
    }

    #[test]
    fn curcinelli_prep_1_array_token() {
        let data = "0721806300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024000000000000000000000000000000000000000000000000000000000000002a00000000000000000000000000000000000000000002888275295397bd1000000000000000000000000000000000000000000000000295be96e640669720000000000000000000000000000000000000000000000000000000000000000000032000000000000000000000000000000000000000000000000000110d9316ec0000000000000000000000000000000000000000000000000000000000000000055000000000000000000000000000000000000000000000000000000000000004600000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000960db77681e940000000000000000000000000000ac93b323cdf11b4a321e4bc933f1cd7a436e79ef0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c0000000000000000000000004c3d5af5c43dbecee525327e93d51eb4d6ddabec00000000000000000000000000000000000000000000000000000000000000010000000000000000000000004c3d5af5c43dbecee525327e93d51eb4d6ddabec";

        let expected_token =
            TokenAddress::from_str("0x4c3d5af5c43dbecee525327e93d51eb4d6ddabec").unwrap();

        let mut data = hex::decode(data).unwrap();
        let token_address = Enemy::extract_token_for_curcinelli_bot(&mut data).unwrap();
        assert_eq!(token_address, expected_token,);
    }

    #[test]
    fn curcinelli_prep_original_tx_1() {
        let data = "917c923b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000028000000000000000000000000000000000000000000000000000000000000002e000000000000000000000000000000000000000000000000000016193a9401d7600000000000000000000000000000000000000000000000000005af3107a400000000000000000000000000000000000000000000000000000000000000000550000000000000000000000000000000000000000000000000000000000000055000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000017fadac133e27e38f80000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000005a0000000000000000000000000000000000000000000000000000000000000028000000000000000000000000f78f28434112a4a256577c026b161ba0c77b41620000000000000000000000005f194ae2a1f7237b7bc50196f1b812f6670acac600000000000000000000000006709d4266bd3ad955ed3ba7ccf898ccb8ec36fb0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c0000000000000000000000002c7aa0f6db781c832340d9375e5b17312ac2610c00000000000000000000000000000000000000000000000000000000000000020000000000000000000000002c7aa0f6db781c832340d9375e5b17312ac2610c000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c";

        let expected_token =
            TokenAddress::from_str("0x2c7aa0f6db781c832340d9375e5b17312ac2610c").unwrap();

        let mut data = hex::decode(data).unwrap();
        let token_address = Enemy::extract_token_for_curcinelli_bot(&mut data).unwrap();
        assert_eq!(token_address, expected_token,);
    }

    #[test]
    fn curcinelli_prep_original_tx_2() {
        let data = "917c923b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000028000000000000000000000000000000000000000000000000000000000000002e000000000000000000000000000000000000000000000000000016193a9401d7600000000000000000000000000000000000000000000000000005af3107a4000000000000000000000000000000000000000000000000000000000000000004b000000000000000000000000000000000000000000000000000000000000004b000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000017e637f8a49c0056370000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000005a00000000000000000000000000000000000000000000000000000000000000280000000000000000000000009797fc7ac33d593585a63ad1c7faa84b5802b3ac000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006709d4266bd3ad955ed3ba7ccf898ccb8ec36fb0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c00000000000000000000000018d3be5ecddf79279004e2d90d507594c2d46f85000000000000000000000000000000000000000000000000000000000000000200000000000000000000000018d3be5ecddf79279004e2d90d507594c2d46f85000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c";

        let expected_token =
            TokenAddress::from_str("0x18d3be5ecddf79279004e2d90d507594c2d46f85").unwrap();

        let mut data = hex::decode(data).unwrap();
        let token_address = Enemy::extract_token_for_curcinelli_bot(&mut data).unwrap();
        assert_eq!(token_address, expected_token,);
    }

    #[test]
    fn curcinelli_prep_original_tx_3() {
        let data = "917c923b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000028000000000000000000000000000000000000000000000000000000000000002e000000000000000000000000000000000000000000000000000016193a9401d7600000000000000000000000000000000000000000000000000005af3107a4000000000000000000000000000000000000000000000000000000000000000005600000000000000000000000000000000000000000000000000000000000000560000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000393ef1a5127c8000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000005a000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000005610da09a54b51a841e2daddc0a2a74c6d37e0b3000000000000000000000000fc4ab3d9076c01e9ba5ed1251cf11ea28107ff6900000000000000000000000006709d4266bd3ad955ed3ba7ccf898ccb8ec36fb0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c000000000000000000000000ae01f96cb9ce103a6a1297cc19ec0d0814cf4c7f0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000ae01f96cb9ce103a6a1297cc19ec0d0814cf4c7f000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c";

        let expected_token =
            TokenAddress::from_str("0xae01f96cb9ce103a6a1297cc19ec0d0814cf4c7f").unwrap();

        let mut data = hex::decode(data).unwrap();
        let token_address = Enemy::extract_token_for_curcinelli_bot(&mut data).unwrap();
        assert_eq!(token_address, expected_token,);
    }
}
