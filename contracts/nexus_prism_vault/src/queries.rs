use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};
use nexus_prism_protocol::vault::{
    ConfigResponse, PotentialRewardsResponse, StakingContract, StateResponse,
    UpdateRewardsDistributionResponse,
};

use crate::{
    commands::update_rewards_distribution,
    reply::{calc_stakers_rewards, vested_prism_balance},
    state::{load_config, load_state},
};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = load_config(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        governance: config.governance.to_string(),

        xprism_token: config.xprism_token.to_string(),
        nexprism_token: config.nexprism_token.to_string(),
        yluna_token: config.yluna_token.to_string(),
        nyluna_token: config.nyluna_token.to_string(),
        prism_token: config.prism_token.to_string(),

        prism_launch_pool: config.prism_launch_pool.to_string(),
        prism_xprism_boost: config.prism_xprism_boost.to_string(),

        nexprism_staking: config.nexprism_staking.to_string(),
        psi_staking: config.psi_staking.to_string(),
        nyluna_staking: config.nyluna_staking.to_string(),

        prism_xprism_pair: config.prism_xprism_pair.to_string(),
        prism_yluna_pair: config.prism_yluna_pair.to_string(),

        rewards_distribution_update_period_secs: config.rewards_distribution_update_period_secs,
        rewards_distribution_update_step: config.rewards_distribution_update_step,

        min_nexprism_stakers_reward_ratio: config.min_nexprism_stakers_reward_ratio,
        max_nexprism_stakers_reward_ratio: config.max_nexprism_stakers_reward_ratio,
        min_nyluna_stakers_reward_ratio: config.min_nyluna_stakers_reward_ratio,
        max_nyluna_stakers_reward_ratio: config.max_nyluna_stakers_reward_ratio,
    })
}

pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = load_state(deps.storage)?;

    Ok(StateResponse {
        nexprism_stakers_reward_ratio: state.nexprism_stakers_reward_ratio,
        nyluna_stakers_reward_ratio: state.nyluna_stakers_reward_ratio,
        psi_stakers_reward_ratio: state.psi_stakers_reward_ratio,
        last_calculation_time: state.last_calculation_time,
        xprism_amount_total: state.xprism_amount_total,
        yluna_amount_total: state.yluna_amount_total,
    })
}

pub fn simulate_update_rewards_distribution(
    deps: Deps,
    env: Env,
) -> StdResult<UpdateRewardsDistributionResponse> {
    let config = load_config(deps.storage)?;
    let state = load_state(deps.storage)?;
    let new_state = update_rewards_distribution(deps, env, &config, &state)?;

    Ok(UpdateRewardsDistributionResponse {
        nexprism_stakers_reward_ratio: new_state.nexprism_stakers_reward_ratio,
        nyluna_stakers_reward_ratio: new_state.nyluna_stakers_reward_ratio,
        psi_stakers_reward_ratio: new_state.psi_stakers_reward_ratio,
    })
}

pub fn query_potential_rewards(
    deps: Deps,
    env: Env,
    staking_contract: StakingContract,
    user_addr: String,
) -> StdResult<PotentialRewardsResponse> {
    let config = load_config(deps.storage)?;
    let state = load_state(deps.storage)?;
    let user_addr = deps.api.addr_validate(&user_addr)?;

    let vested_prism_balance = vested_prism_balance(deps, &env, &config.prism_launch_pool)?;
    let (nexprism_stakers_rewards, nyluna_stakers_rewards, psi_stakers_rewards) =
        calc_stakers_rewards(&state, vested_prism_balance);

    let rewards = match staking_contract {
        StakingContract::NexPrism {} => query_staking_potential_rewards(
            deps,
            &config.nexprism_staking,
            nexprism_stakers_rewards,
            &user_addr,
        )?,
        StakingContract::NyLuna {} => query_staking_potential_rewards(
            deps,
            &config.nyluna_staking,
            nyluna_stakers_rewards,
            &user_addr,
        )?,
        StakingContract::Psi {} => query_staking_potential_rewards(
            deps,
            &config.psi_staking,
            psi_stakers_rewards,
            &user_addr,
        )?,
    };

    Ok(PotentialRewardsResponse { rewards })
}

fn query_staking_potential_rewards(
    deps: Deps,
    staking_addr: &Addr,
    potential_rewards: Uint128,
    address: &Addr,
) -> StdResult<Uint128> {
    let response: nexus_prism_protocol::staking::PotentialRewardsResponse =
        deps.querier.query_wasm_smart(
            staking_addr,
            &nexus_prism_protocol::staking::QueryMsg::GetPotentialRewards {
                potential_rewards,
                address: address.into(),
            },
        )?;
    Ok(response.rewards)
}
