use crate::{
    commands::{calculate_global_index, get_staker_balance, get_staking_total_balance},
    state::State,
    utils::calculate_decimal_rewards,
};
use cosmwasm_std::{Decimal, Deps, Env, StdResult, Uint128};
use nexus_prism_protocol::{
    common::{query_token_balance, sum},
    staking::{
        ConfigResponse, PotentialRewardsResponse, RewardsResponse, StakerResponse, StateResponse,
    },
};

use crate::state::{load_config, load_staker, load_state, Config};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = load_config(deps.storage)?;

    Ok(ConfigResponse {
        governance: config.governance.to_string(),
        staking_token: config.staking_token.to_string(),
        stake_operator: config.stake_operator.map(|addr| addr.to_string()),
        reward_token: config.reward_token.to_string(),
        reward_operator: config.reward_operator.to_string(),
        xprism_token: config.xprism_token.map(|addr| addr.to_string()),
        prism_governance: config.prism_governance.map(|addr| addr.to_string()),
        nexprism_xprism_pair: config.nexprism_xprism_pair.map(|addr| addr.to_string()),
    })
}

pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = load_state(deps.storage)?;

    Ok(StateResponse {
        staking_total_balance: state.staking_total_balance,
        virtual_reward_balance: state.virtual_reward_balance,
    })
}

pub fn query_rewards(deps: Deps, address: String) -> StdResult<RewardsResponse> {
    let config = load_config(deps.storage)?;

    let staker_addr = deps.api.addr_validate(&address)?;
    let mut staker = load_staker(deps.storage, &staker_addr)?;
    staker.balance = get_staker_balance(deps, config.stake_operator, &staker, &staker_addr)?;

    let real_global_index = load_state(deps.storage)?.real_rewards.global_index;
    let real_reward_with_decimals =
        calculate_decimal_rewards(real_global_index, staker.real_index, staker.balance)?;
    let all_real_reward_with_decimals = sum(real_reward_with_decimals, staker.real_pending_rewards);
    let real_rewards = all_real_reward_with_decimals * Uint128::new(1);

    let virtual_global_index = load_state(deps.storage)?.virtual_rewards.global_index;
    let virtual_reward_with_decimals =
        calculate_decimal_rewards(virtual_global_index, staker.virtual_index, staker.balance)?;
    let all_virtual_reward_with_decimals =
        sum(virtual_reward_with_decimals, staker.virtual_pending_rewards);
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

    state.staking_total_balance =
        get_staking_total_balance(deps, config.stake_operator.clone(), &state)?;
    staker.balance = get_staker_balance(deps, config.stake_operator, &staker, &staker_addr)?;

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
        sum(virtual_reward_with_decimals, staker.virtual_pending_rewards);
    staker.virtual_pending_rewards = all_virtual_reward_with_decimals;
    staker.virtual_index = state.virtual_rewards.global_index;

    let real_reward_with_decimals = calculate_decimal_rewards(
        state.real_rewards.global_index,
        staker.real_index,
        staker.balance,
    )?;
    let all_real_reward_with_decimals: Decimal =
        sum(real_reward_with_decimals, staker.real_pending_rewards);
    staker.real_pending_rewards = all_real_reward_with_decimals;
    staker.real_index = state.real_rewards.global_index;

    Ok(StakerResponse {
        address,
        balance: staker.balance,
        virtual_pending_rewards: staker.virtual_pending_rewards,
        real_pending_rewards: staker.real_pending_rewards,
    })
}

pub fn query_potential_rewards(
    deps: Deps,
    _env: Env,
    potential_rewards_total: Uint128,
    address: String,
) -> StdResult<PotentialRewardsResponse> {
    let mut state = load_state(deps.storage)?;

    if state.staking_total_balance.is_zero() {
        return Ok(PotentialRewardsResponse {
            rewards: Uint128::zero(),
        });
    }

    calculate_global_index(
        potential_rewards_total,
        state.staking_total_balance,
        &mut state.real_rewards,
    )?;

    let staker_addr = deps.api.addr_validate(&address)?;
    let staker = load_staker(deps.storage, &staker_addr)?;

    let real_global_index = state.real_rewards.global_index;
    let real_reward_with_decimals =
        calculate_decimal_rewards(real_global_index, staker.real_index, staker.balance)?;
    let all_real_reward_with_decimals = sum(real_reward_with_decimals, staker.real_pending_rewards);
    let real_rewards = all_real_reward_with_decimals * Uint128::new(1);

    Ok(PotentialRewardsResponse {
        rewards: real_rewards,
    })
}
