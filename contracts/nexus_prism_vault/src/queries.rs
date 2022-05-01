use cosmwasm_std::{Deps, StdResult};
use nexus_prism_protocol::vault::ConfigResponse;

use crate::state::load_config;

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

        nexprism_xprism_pair: config.nexprism_xprism_pair.to_string(),
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
