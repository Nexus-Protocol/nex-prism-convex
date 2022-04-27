use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub governance: Addr,
    pub psi_token: Addr,
    pub xprism_token: Addr,
    pub nexprism_token: Addr,
    pub yluna_token: Addr,
    pub nyluna_token: Addr,
    pub prism_token: Addr,
    pub prism_launch_pool: Addr,
    pub prism_xprism_boost: Addr,
    pub astroport_factory: Addr,
    pub staking_code_id: u64,
    pub staking_admin: Addr,
    pub nexprism_xprism_staking: Addr,
    pub psi_nexprism_staking: Addr,
    pub yluna_prism_staking: Addr,
    pub xprism_nexprism_pair: Addr,
    pub xprism_prism_pair: Addr,
    pub yluna_prism_pair: Addr,
    pub rewards_distribution_update_period: Option<u64>,
    pub rewards_distribution_update_step: Decimal,
    pub min_nexprism_stakers_reward_ratio: Decimal,
    pub max_nexprism_stakers_reward_ratio: Decimal,
    pub min_yluna_depositors_reward_ratio: Decimal,
    pub max_yluna_depositors_reward_ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    pub nexprism_stakers_reward_ratio: Decimal,
    pub yluna_depositors_reward_ratio: Decimal,
    pub psi_stakers_reward_ratio: Decimal,
    pub last_calculation_time: u64,
}

pub fn save_state(
    store: &mut dyn Storage,
    config: &Config,
    state: &State,
) -> Result<(), ContractError> {
    if state.nexprism_stakers_reward_ratio
        + state.yluna_depositors_reward_ratio
        + state.psi_stakers_reward_ratio
        != Decimal::one()
        || state.nexprism_stakers_reward_ratio > config.max_nexprism_stakers_reward_ratio
        || state.nexprism_stakers_reward_ratio < config.min_nexprism_stakers_reward_ratio
        || state.yluna_depositors_reward_ratio > config.max_yluna_depositors_reward_ratio
        || state.yluna_depositors_reward_ratio < config.min_yluna_depositors_reward_ratio
    {
        return Err(ContractError::InvalidRewardRatios {});
    }

    STATE.save(store, state)?;

    Ok(())
}

pub fn set_nyluna(storage: &mut dyn Storage, addr: Addr) -> StdResult<Config> {
    CONFIG.update(storage, |mut config: Config| -> StdResult<_> {
        config.nyluna_token = addr;
        Ok(config)
    })
}

pub fn set_nexprism(storage: &mut dyn Storage, addr: Addr) -> StdResult<Config> {
    CONFIG.update(storage, |mut config: Config| -> StdResult<_> {
        config.nexprism_token = addr;
        Ok(config)
    })
}

pub fn set_nexprism_staking(storage: &mut dyn Storage, addr: Addr) -> StdResult<Config> {
    CONFIG.update(storage, |mut config: Config| -> StdResult<_> {
        config.nexprism_xprism_staking = addr;
        Ok(config)
    })
}

pub fn set_yluna_staking(storage: &mut dyn Storage, addr: Addr) -> StdResult<Config> {
    CONFIG.update(storage, |mut config: Config| -> StdResult<_> {
        config.yluna_prism_staking = addr;
        Ok(config)
    })
}

pub fn set_psi_staking(storage: &mut dyn Storage, addr: Addr) -> StdResult<Config> {
    CONFIG.update(storage, |mut config: Config| -> StdResult<_> {
        config.psi_nexprism_staking = addr;
        Ok(config)
    })
}

pub fn set_xprism_nexprism_pair(storage: &mut dyn Storage, addr: Addr) -> StdResult<Config> {
    CONFIG.update(storage, |mut config: Config| -> StdResult<_> {
        config.xprism_nexprism_pair = addr;
        Ok(config)
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GovernanceUpdateState {
    pub new_governance: Addr,
    pub wait_approve_until: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReplyContext {
    pub reward_balance: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const STATE: Item<State> = Item::new("state");

pub const REPLY_CONTEXT: Item<ReplyContext> = Item::new("reply");

pub const GOVERNANCE_UPDATE: Item<GovernanceUpdateState> = Item::new("gov_update");
