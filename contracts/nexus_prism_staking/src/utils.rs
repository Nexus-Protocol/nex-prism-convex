use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};
use nexus_prism_protocol::common::{mul, sub, sum};
use std::str::FromStr;

// calculate the reward based on the user index and the global index.
pub fn calculate_decimal_rewards(
    global_index: Decimal,
    user_index: Decimal,
    user_balance: Uint128,
) -> StdResult<Decimal> {
    let decimal_balance = Decimal::from_ratio(user_balance, Uint128::new(1));
    Ok(mul(sub(global_index, user_index), decimal_balance))
}

// calculate the reward with decimal
pub fn get_decimals(value: Decimal) -> StdResult<Decimal> {
    let stringed: &str = &*value.to_string();
    let parts: &[&str] = &*stringed.split('.').collect::<Vec<&str>>();
    match parts.len() {
        1 => Ok(Decimal::zero()),
        2 => {
            let decimals = Decimal::from_str(&*("0.".to_owned() + parts[1]))?;
            Ok(decimals)
        }
        _ => Err(StdError::generic_err("Unexpected number of dots")),
    }
}

pub fn decimal_minus_uint128(decimal: Decimal, uint128: Uint128) -> Decimal {
    decimal - Decimal::from_ratio(uint128, Uint128::new(1))
}

pub fn sum_decimals_and_split_result_to_uint_and_decimal(
    d1: Decimal,
    d2: Decimal,
) -> StdResult<(Uint128, Decimal)> {
    let sum = sum(d1, d2);
    let decimals: Decimal = get_decimals(sum)?;
    let uint128: Uint128 = sum * Uint128::new(1);
    Ok((uint128, decimals))
}

pub fn substract_and_update(field: &mut Uint128, to_sub: &Uint128) {
    let result = *field - *to_sub;
    *field = result;
}

#[cfg(test)]
mod tests {
    use crate::state::{RewardState, State};

    use super::*;

    #[test]
    pub fn proper_calculate_rewards() {
        let global_index = Decimal::from_ratio(Uint128::new(9), Uint128::new(100));
        let user_index = Decimal::zero();
        let user_balance = Uint128::new(1000);
        let reward = calculate_decimal_rewards(global_index, user_index, user_balance).unwrap();
        assert_eq!(reward.to_string(), "90");
    }

    #[test]
    pub fn proper_get_decimals() {
        let global_index = Decimal::from_ratio(Uint128::new(9999999), Uint128::new(100000000));
        let user_index = Decimal::zero();
        let user_balance = Uint128::new(10);
        let reward = get_decimals(
            calculate_decimal_rewards(global_index, user_index, user_balance).unwrap(),
        )
        .unwrap();
        assert_eq!(reward.to_string(), "0.9999999");
    }

    #[test]
    pub fn sub_and_update_1() {
        let _1000 = Uint128::from(1_000u128);
        let _2500 = Uint128::from(2_500u128);
        let rewards = _1000;

        let mut state = State {
            staking_total_balance: Uint128::zero(),
            virtual_reward_balance: Uint128::zero(),
            virtual_rewards: RewardState {
                global_index: Decimal::zero(),
                prev_balance: _2500,
            },
            real_rewards: RewardState {
                global_index: Decimal::zero(),
                prev_balance: _2500,
            },
        };

        substract_and_update(&mut state.real_rewards.prev_balance, &rewards);
        substract_and_update(&mut state.virtual_rewards.prev_balance, &rewards);
        //2500 - 1000 = 1500
        assert_eq!(state.real_rewards.prev_balance, _2500 - _1000);
        assert_eq!(state.virtual_rewards.prev_balance, _2500 - _1000);
    }
}
