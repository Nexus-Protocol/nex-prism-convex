use crate::{
    commands::calculate_global_index, math::decimal_summation_in_256, state::State,
    utils::calculate_decimal_rewards,
};
use cosmwasm_std::{Decimal, Deps, Env, StdResult, Uint128};
use nexus_prism_protocol::{
    common::query_token_balance,
    staking::{ConfigResponse, RewardsResponse, StakerResponse},
};

use crate::state::{load_config, load_staker, load_state, Config};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = load_config(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner.map(|addr| addr.to_string()),
        staking_token: config.staking_token.to_string(),
        rewarder: config.rewarder.to_string(),
        reward_token: config.reward_token.to_string(),
        staker_reward_pair: config
            .staker_reward_pair
            .iter()
            .map(|p| p.to_string())
            .collect(),
        governance: config.governance.to_string(),
    })
}

pub fn query_rewards(deps: Deps, address: String) -> StdResult<RewardsResponse> {
    let staker_addr = deps.api.addr_validate(&address)?;
    let staker = load_staker(deps.storage, &staker_addr)?;

    let real_global_index = load_state(deps.storage)?.real_rewards.global_index;
    let real_reward_with_decimals =
        calculate_decimal_rewards(real_global_index, staker.real_index, staker.balance)?;
    let all_real_reward_with_decimals =
        decimal_summation_in_256(real_reward_with_decimals, staker.real_pending_rewards);
    let real_rewards = all_real_reward_with_decimals * Uint128::new(1);

    let virtual_global_index = load_state(deps.storage)?.virtual_rewards.global_index;
    let virtual_reward_with_decimals =
        calculate_decimal_rewards(virtual_global_index, staker.virtual_index, staker.balance)?;
    let all_virtual_reward_with_decimals =
        decimal_summation_in_256(virtual_reward_with_decimals, staker.virtual_pending_rewards);
    let virtual_rewards = all_virtual_reward_with_decimals * Uint128::new(1);

    Ok(RewardsResponse {
        virtual_rewards,
        real_rewards,
    })
}

pub fn query_staker(deps: Deps, env: Env, address: String) -> StdResult<StakerResponse> {
    let staker_addr = deps.api.addr_validate(&address)?;
    let mut staker = load_staker(deps.storage, &staker_addr)?;

    let mut state: State = load_state(deps.storage)?;
    let config: Config = load_config(deps.storage)?;

    calculate_global_index(
        state.virtual_reward_balance,
        state.staking_total_balance,
        &mut state.virtual_rewards,
    )?;
    calculate_global_index(
        query_token_balance(deps, &config.reward_token, &env.contract.address),
        state.staking_total_balance,
        &mut state.real_rewards,
    )?;

    let virtual_reward_with_decimals = calculate_decimal_rewards(
        state.virtual_rewards.global_index,
        staker.virtual_index,
        staker.balance,
    )?;
    let all_virtual_reward_with_decimals: Decimal =
        decimal_summation_in_256(virtual_reward_with_decimals, staker.virtual_pending_rewards);
    staker.virtual_pending_rewards = all_virtual_reward_with_decimals;
    staker.virtual_index = state.virtual_rewards.global_index;

    let real_reward_with_decimals = calculate_decimal_rewards(
        state.real_rewards.global_index,
        staker.real_index,
        staker.balance,
    )?;
    let all_real_reward_with_decimals: Decimal =
        decimal_summation_in_256(real_reward_with_decimals, staker.real_pending_rewards);
    staker.real_pending_rewards = all_real_reward_with_decimals;
    staker.real_index = state.real_rewards.global_index;

    Ok(StakerResponse {
        address,
        balance: staker.balance,
        virtual_pending_rewards: staker.virtual_pending_rewards,
        real_pending_rewards: staker.real_pending_rewards,
    })
}
