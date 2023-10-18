use std::ops::RangeInclusive;

use ethers::types::U256;

const MIN_GAS_PRICE: u64 = 3;
const MAX_SUPPORTED_GAS_PRICE: u64 = 15;
const DEFAULT_GWEI_PRECISION: usize = 9;

/// This means that we support up to 3 decimal places for gwei_to_wei
/// eg. 1.123 gwei  is supported but 1.1234 gwei is not
pub const DEFAULT_GWEI_DECIMAL_PRECISION: usize = 3;

pub fn gwei_to_wei(gwei: u64) -> U256 {
    U256::from(gwei) * U256::exp10(DEFAULT_GWEI_PRECISION)
}

pub fn gwei_to_wei_with_decimals(gwei: u64, decimal_precision: usize) -> U256 {
    U256::from(gwei) * U256::exp10(DEFAULT_GWEI_PRECISION - decimal_precision)
}

pub fn wei_to_gwei_no_decimals(wei: U256) -> usize {
    let gwei = wei / U256::exp10(DEFAULT_GWEI_PRECISION);
    gwei.as_usize()
}

pub fn wei_to_gwei_with_decimals(wei: U256, decimal_precision: usize) -> usize {
    let gwei = wei / U256::exp10(DEFAULT_GWEI_PRECISION - decimal_precision);
    gwei.as_usize()
}

pub fn gwei_to_index(wei: U256, decimal_precision: usize) -> usize {
    let minimal_gwei = (U256::from(MIN_GAS_PRICE) * U256::exp10(decimal_precision)).as_usize();
    let gwei = (wei / U256::exp10(DEFAULT_GWEI_PRECISION - decimal_precision)).as_usize();
    gwei - minimal_gwei
}

pub fn get_default_gas_price_range() -> RangeInclusive<u64> {
    get_gas_price_range(MAX_SUPPORTED_GAS_PRICE, DEFAULT_GWEI_DECIMAL_PRECISION)
}

pub fn get_gas_price_range(
    max_supported_gas_price: u64,
    decimal_precision: usize,
) -> RangeInclusive<u64> {
    MIN_GAS_PRICE * 10u64.pow(decimal_precision as u32)
        ..=max_supported_gas_price * 10u64.pow(decimal_precision as u32)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn wei_to_gwei_test() {
        assert_eq!(wei_to_gwei_no_decimals(U256::from(3000000000usize)), 3);
        assert_eq!(wei_to_gwei_no_decimals(U256::from(3010000000usize)), 3);
        assert_eq!(wei_to_gwei_no_decimals(U256::from(30000000000usize)), 30);

        assert_eq!(
            wei_to_gwei_with_decimals(U256::from(3010000000usize), 2),
            301
        );

        assert_eq!(
            wei_to_gwei_with_decimals(U256::from(3000000000usize), 5),
            300000
        );

        assert_eq!(
            wei_to_gwei_with_decimals(U256::from(3010000000usize), 4),
            30100
        );

        assert_eq!(
            wei_to_gwei_with_decimals(U256::from(301010000000usize), 2),
            30101
        );
    }

    #[test]
    fn get_index_test() {
        // index of 3gwei is always 0
        assert_eq!(gwei_to_index(U256::from(3000000000usize), 2), 0);
        assert_eq!(gwei_to_index(U256::from(3000000000usize), 5), 0);
        assert_eq!(gwei_to_index(U256::from(3000000000usize), 9), 0);

        assert_eq!(gwei_to_index(U256::from(5000000000usize), 0), 2);

        assert_eq!(gwei_to_index(U256::from(3010000000usize), 2), 1);
        assert_eq!(gwei_to_index(U256::from(5001000000usize), 3), 2001);
    }

    #[test]
    fn gas_price_range_test() {
        let range = get_default_gas_price_range();
        assert_eq!(range.start(), &3_000);
        assert_eq!(range.end(), &15_000);

        let mut test_vec = Vec::with_capacity((range.end() - range.start() + 1) as usize);
        for i in range {
            test_vec.push(i);
        }

        assert_eq!(test_vec.len(), 12_001);
        let test_gas_price = 5001000000usize;
        assert_eq!(test_vec[gwei_to_index(U256::from(test_gas_price), 3)], 5001);
        assert_eq!(
            gwei_to_wei_with_decimals(
                test_vec[gwei_to_index(U256::from(test_gas_price), DEFAULT_GWEI_DECIMAL_PRECISION)],
                DEFAULT_GWEI_DECIMAL_PRECISION
            ),
            test_gas_price.into()
        );
    }
}
