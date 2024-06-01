use std::ops::{Add, Div, Mul, Sub};

use drift::{
    math::constants::{BID_ASK_SPREAD_PRECISION, FIVE_MINUTE},
    state::{
        oracle::{HistoricalOracleData, OraclePriceData},
        perp_market::AMM,
    },
};

pub fn calculate_live_oracle_twap(
    hist_oracle_data: &HistoricalOracleData,
    oracle_price_data: &OraclePriceData,
    now: i64,
    period: i64,
) -> i64 {
    let oracle_twap = if period == FIVE_MINUTE as i64 {
        hist_oracle_data.last_oracle_price_twap_5min
    } else {
        hist_oracle_data.last_oracle_price_twap
    };

    let since_last_update = std::cmp::max(1, now - hist_oracle_data.last_oracle_price_twap_ts);
    let since_start = std::cmp::max(0, period - since_last_update);

    let clamp_range = oracle_twap / 3;

    let clamped_oracle_price = std::cmp::min(
        oracle_twap + clamp_range,
        std::cmp::max(oracle_price_data.price, oracle_twap - clamp_range),
    );

    (oracle_twap)
        .mul(since_start)
        .add((clamped_oracle_price).mul(since_last_update))
        .div(since_start.add(since_last_update))
}

pub fn calculate_live_oracle_std(amm: &AMM, oracle_price_data: &OraclePriceData, now: i64) -> i64 {
    let since_last_update = std::cmp::max(
        1,
        now.sub(amm.historical_oracle_data.last_oracle_price_twap_ts),
    );
    let since_start = std::cmp::max(0, (amm.funding_period).sub(since_last_update));

    let live_oracle_twap = calculate_live_oracle_twap(
        &amm.historical_oracle_data,
        oracle_price_data,
        now,
        amm.funding_period,
    );

    let price_delta_vs_twap = oracle_price_data.price.sub(live_oracle_twap).abs();

    price_delta_vs_twap.add(
        (amm.oracle_std as i64)
            .mul(since_start)
            .div(since_start.add(since_last_update)),
    )
}

pub fn get_new_oracle_conf_pct(
    amm: &AMM,
    oracle_price_data: &OraclePriceData,
    reserve_price: u64,
    now: i64,
) -> u64 {
    let conf_interval = oracle_price_data.confidence;

    let since_last_update = std::cmp::max(
        0,
        now.sub(amm.historical_oracle_data.last_oracle_price_twap_ts),
    );
    let mut lower_bound_conf_pct = amm.last_oracle_conf_pct;
    if since_last_update > 0 {
        let lower_bound_conf_divisor = std::cmp::max(21.sub(since_last_update), 5);
        lower_bound_conf_pct =
            amm.last_oracle_conf_pct - amm.last_oracle_conf_pct / lower_bound_conf_divisor as u64;
    }
    let conf_interval_pct = conf_interval * BID_ASK_SPREAD_PRECISION / reserve_price;

    std::cmp::max(conf_interval_pct, lower_bound_conf_pct)
}

#[cfg(test)]
mod tests {
    use drift::{
        math::constants::FIVE_MINUTE,
        state::{
            oracle::{HistoricalOracleData, OraclePriceData},
            perp_market::AMM,
        },
    };

    use super::{calculate_live_oracle_std, calculate_live_oracle_twap};

    #[test]
    fn test_calculate_live_oracle_twap() {
        let hist_data = HistoricalOracleData {
            last_oracle_price_twap: 1000,
            last_oracle_price_twap_5min: 1200,
            last_oracle_price_twap_ts: 100,
            ..Default::default()
        };
        let oracle_data = OraclePriceData {
            price: 1100,
            ..Default::default()
        };

        let now = 200;
        let period = FIVE_MINUTE as i64;

        let result = calculate_live_oracle_twap(&hist_data, &oracle_data, now, period);

        assert_eq!(
            1166, result,
            "The TWAP calculation did not return the expected value"
        );
    }

    #[test]
    fn test_calculate_live_oracle_twap_long_period() {
        let hist_data = HistoricalOracleData {
            last_oracle_price_twap: 1000,
            last_oracle_price_twap_5min: 1200,
            last_oracle_price_twap_ts: 100,
            ..Default::default()
        };
        let oracle_data = OraclePriceData {
            price: 800,
            ..Default::default()
        };

        let now = 200; // Current timestamp
        let period = 1000; // longer period

        let result = calculate_live_oracle_twap(&hist_data, &oracle_data, now, period);

        assert_eq!(
            980, result,
            "The TWAP calculation for longer periods did not return the expected value"
        );
    }

    #[test]
    fn test_calculate_live_oracle_std() {
        let hist_data = HistoricalOracleData {
            last_oracle_price_twap: 1000,
            last_oracle_price_twap_5min: 1200,
            last_oracle_price_twap_ts: 100,
            ..Default::default()
        };
        let amm = AMM {
            historical_oracle_data: hist_data,
            funding_period: FIVE_MINUTE as i64,
            oracle_std: 5,
            ..Default::default()
        };
        let oracle_data = OraclePriceData {
            price: 800,
            ..Default::default()
        };
        let now = 200;

        let result = calculate_live_oracle_std(&amm, &oracle_data, now);

        assert_eq!(
            269, result,
            "The calculated standard deviation did not match the expected value."
        );
    }
}
