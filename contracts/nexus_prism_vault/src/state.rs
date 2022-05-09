use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;

const CONFIG: Item<Config> = Item::new("config");
pub const INST_CONFIG: Item<InstantiationConfig> = Item::new("inst_config");
const STATE: Item<State> = Item::new("state");
const CLAIM_VIRTUAL_REWARDS_REPLY_CONTEXT: Item<ClaimVirtualRewardsReplyContext> =
    Item::new("claim_virt_rewards_reply_ctx");
const PRISM_VESTING_STATE: Item<PrismVestingState> = Item::new("prism_vesting_state");
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
    pub nexprism_xprism_pair: Addr,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PrismVestingSchedule {
    pub end_time: u64,
    pub amount: Uint128,
}

impl From<(u64, Uint128)> for PrismVestingSchedule {
    fn from(value: (u64, Uint128)) -> Self {
        Self {
            end_time: value.0,
            amount: value.1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PrismVestingState {
    pub schedules: Vec<PrismVestingSchedule>,
}

pub fn load_prism_vesting_schedules(store: &dyn Storage) -> StdResult<Vec<PrismVestingSchedule>> {
    Ok(PRISM_VESTING_STATE.load(store)?.schedules)
}

pub fn may_load_prism_vesting_schedules(
    store: &dyn Storage,
) -> StdResult<Option<Vec<PrismVestingSchedule>>> {
    Ok(PRISM_VESTING_STATE
        .may_load(store)?
        .map(|state| state.schedules))
}

pub fn save_prism_vesting_schedules(
    store: &mut dyn Storage,
    schedules: Vec<PrismVestingSchedule>,
) -> StdResult<()> {
    PRISM_VESTING_STATE.save(store, &PrismVestingState { schedules })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ClaimVirtualRewardsReplyContext {
    pub locked_vested_prism_amount: Uint128,
}

pub fn load_locked_vested_prism_amount(store: &dyn Storage) -> StdResult<Uint128> {
    Ok(CLAIM_VIRTUAL_REWARDS_REPLY_CONTEXT
        .load(store)?
        .locked_vested_prism_amount)
}

pub fn save_locked_vested_prism_amount(store: &mut dyn Storage, amount: Uint128) -> StdResult<()> {
    CLAIM_VIRTUAL_REWARDS_REPLY_CONTEXT.save(
        store,
        &ClaimVirtualRewardsReplyContext {
            locked_vested_prism_amount: amount,
        },
    )
}
