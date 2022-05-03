use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;

const CONFIG: Item<Config> = Item::new("config");
pub const INST_CONFIG: Item<InstantiationConfig> = Item::new("inst_config");
const STATE: Item<State> = Item::new("state");
pub const REPLY_CONTEXT: Item<ReplyContext> = Item::new("reply");
pub const GOVERNANCE_UPDATE: Item<GovernanceUpdateState> = Item::new("gov_update");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub governance: Addr,

    pub xprism_token: Addr,
    pub nexprism_token: Addr,
    pub yluna_token: Addr,
    pub nyluna_token: Addr,
    pub prism_token: Addr,

    pub prism_launch_pool: Addr,
    pub prism_xprism_boost: Addr,

    pub nexprism_staking: Addr,
    pub psi_staking: Addr,
    pub nyluna_staking: Addr,

    pub nexprism_xprism_pair: Addr,
    pub prism_xprism_pair: Addr,
    pub prism_yluna_pair: Addr,

    pub rewards_distribution_update_period_secs: Option<u64>,
    pub rewards_distribution_update_step: Decimal,

    pub min_nexprism_stakers_reward_ratio: Decimal,
    pub max_nexprism_stakers_reward_ratio: Decimal,
    pub min_nyluna_stakers_reward_ratio: Decimal,
    pub max_nyluna_stakers_reward_ratio: Decimal,
}

pub fn load_config(store: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(store)
}

pub fn save_config(store: &mut dyn Storage, config: &Config) -> Result<(), ContractError> {
    if config.rewards_distribution_update_step.is_zero()
        || config.min_nexprism_stakers_reward_ratio > Decimal::one()
        || config.max_nexprism_stakers_reward_ratio > Decimal::one()
        || config.min_nyluna_stakers_reward_ratio > Decimal::one()
        || config.max_nyluna_stakers_reward_ratio > Decimal::one()
        || config.min_nexprism_stakers_reward_ratio >= config.max_nexprism_stakers_reward_ratio
        || config.min_nyluna_stakers_reward_ratio >= config.max_nyluna_stakers_reward_ratio
    {
        return Err(ContractError::InvalidConfig {});
    }

    CONFIG.save(store, config)?;

    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiationConfig {
    pub admin: Addr,
    pub cw20_token_code_id: u64,
    pub staking_code_id: u64,
    pub autocompounder_code_id: u64,
    pub astroport_factory: Addr,
    pub nexprism_xprism_amp_coef: u64,
    pub psi_token: Addr,
    pub prism_governance: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    pub nexprism_stakers_reward_ratio: Decimal,
    pub nyluna_stakers_reward_ratio: Decimal,
    pub psi_stakers_reward_ratio: Decimal,
    pub last_calculation_time: u64,
    pub xprism_amount_total: Uint128,
    pub yluna_amount_total: Uint128,
}

pub fn load_state(store: &dyn Storage) -> StdResult<State> {
    STATE.load(store)
}

pub fn save_state(
    store: &mut dyn Storage,
    config: &Config,
    state: &State,
) -> Result<(), ContractError> {
    if state.nexprism_stakers_reward_ratio
        + state.nyluna_stakers_reward_ratio
        + state.psi_stakers_reward_ratio
        != Decimal::one()
        || state.nexprism_stakers_reward_ratio > config.max_nexprism_stakers_reward_ratio
        || state.nexprism_stakers_reward_ratio < config.min_nexprism_stakers_reward_ratio
        || state.nyluna_stakers_reward_ratio > config.max_nyluna_stakers_reward_ratio
        || state.nyluna_stakers_reward_ratio < config.min_nyluna_stakers_reward_ratio
    {
        return Err(ContractError::InvalidState {});
    }

    STATE.save(store, state)?;

    Ok(())
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
