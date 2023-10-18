use ethers::types::U256;

const DEFAULT_GWEI_PRECISION: usize = 9;

/// This means that we support up to 3 decimal places for gwei_to_wei
/// eg. 1.123 gwei  is supported but 1.1234 gwei is not
const DEFAULT_GWEI_DECIMAL_PRECISION: usize = 3;

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
    let minimal_gwei = (U256::from(3) * U256::exp10(decimal_precision)).as_usize();
    let gwei = (wei / U256::exp10(DEFAULT_GWEI_PRECISION - decimal_precision)).as_usize();
    gwei - minimal_gwei
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
}
