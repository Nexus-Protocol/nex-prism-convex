use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};

const KEY_CONFIG: Item<Config> = Item::new("config");
const KEY_STATE: Item<State> = Item::new("state");
const KEY_GOVERNANCE_UPDATE: Item<GovernanceUpdateState> = Item::new("gov_update");
pub const STAKERS: Map<&Addr, Staker> = Map::new("state");
pub const REPLY_CONTEXT: Item<ReplyContext> = Item::new("reply");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub governance: Addr,
    pub staking_token: Addr,
    pub stake_operator: Option<Addr>,
    pub reward_token: Addr,
    pub reward_operator: Addr,
    pub xprism_token: Option<Addr>,
    pub prism_governance: Option<Addr>,
    pub nexprism_xprism_pair: Option<Addr>,
}

impl Config {
    pub fn with_stake_operator(&self) -> bool {
        self.stake_operator.is_some()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    pub staking_total_balance: Uint128,
    pub virtual_reward_balance: Uint128,
    pub virtual_rewards: RewardState,
    pub real_rewards: RewardState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RewardState {
    pub global_index: Decimal,
    pub prev_balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReplyContext {
    pub rewards_recipient: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Staker {
    pub balance: Uint128,
    pub real_index: Decimal,
    pub real_pending_rewards: Decimal,
    pub virtual_index: Decimal,
    pub virtual_pending_rewards: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GovernanceUpdateState {
    pub new_governance_contract_addr: Addr,
    pub wait_approve_until: u64,
}

pub fn load_state(storage: &dyn Storage) -> StdResult<State> {
    KEY_STATE.load(storage)
}

pub fn save_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    KEY_STATE.save(storage, state)
}

pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    KEY_CONFIG.load(storage)
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    KEY_CONFIG.save(storage, config)
}

pub fn load_staker(storage: &dyn Storage, addr: &Addr) -> StdResult<Staker> {
    STAKERS
        .may_load(storage, addr)
        .map(|res| res.unwrap_or_default())
}

pub fn save_staker(storage: &mut dyn Storage, addr: &Addr, holder: &Staker) -> StdResult<()> {
    STAKERS.save(storage, addr, holder)
}

pub fn load_gov_update(storage: &dyn Storage) -> StdResult<GovernanceUpdateState> {
    KEY_GOVERNANCE_UPDATE.load(storage)
}

pub fn save_gov_update(
    storage: &mut dyn Storage,
    gov_update: &GovernanceUpdateState,
) -> StdResult<()> {
    KEY_GOVERNANCE_UPDATE.save(storage, gov_update)
}

pub fn remove_gov_update(storage: &mut dyn Storage) {
    KEY_GOVERNANCE_UPDATE.remove(storage)
}
