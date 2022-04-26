use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdError, StdResult, Storage, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub compounding_token: Addr,
    pub auto_compounding_token: Addr,
    pub reward_token: Addr,
    pub reward_compound_pair: Addr,
    pub governance_contract: Addr,
    pub rewards_contract: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GovernanceUpdateState {
    pub new_governance_contract_addr: Addr,
    pub wait_approve_until: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WithdrawAction {
    pub farmer: Addr,
    pub auto_compounding_token_amount: Uint128,
}

static KEY_CONFIG: Item<Config> = Item::new("config");
static KEY_WITHDRAW_ACTION: Item<Option<WithdrawAction>> = Item::new("withdraw_action");

static KEY_GOVERNANCE_UPDATE: Item<GovernanceUpdateState> = Item::new("gov_update");

pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    KEY_CONFIG.load(storage)
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    KEY_CONFIG.save(storage, config)
}

pub fn load_withdraw_action(storage: &dyn Storage) -> StdResult<Option<WithdrawAction>> {
    KEY_WITHDRAW_ACTION.load(storage)
}

pub fn store_withdraw_action(
    storage: &mut dyn Storage,
    withdraw_action: WithdrawAction,
) -> StdResult<()> {
    KEY_WITHDRAW_ACTION.update(storage, |v| {
        if v.is_some() {
            Err(StdError::generic_err("Repetitive reply definition!"))
        } else {
            Ok(Some(withdraw_action))
        }
    })?;
    Ok(())
}

pub fn remove_withdraw_action(storage: &mut dyn Storage) -> StdResult<()> {
    KEY_WITHDRAW_ACTION.save(storage, &None)
}

pub fn config_set_nasset_token(storage: &mut dyn Storage, nasset_token: Addr) -> StdResult<Config> {
    KEY_CONFIG.update(storage, |mut config: Config| -> StdResult<_> {
        config.compounding_token = nasset_token;
        Ok(config)
    })
}

pub fn load_gov_update(storage: &dyn Storage) -> StdResult<GovernanceUpdateState> {
    KEY_GOVERNANCE_UPDATE.load(storage)
}

pub fn store_gov_update(
    storage: &mut dyn Storage,
    gov_update: &GovernanceUpdateState,
) -> StdResult<()> {
    KEY_GOVERNANCE_UPDATE.save(storage, gov_update)
}

pub fn remove_gov_update(storage: &mut dyn Storage) {
    KEY_GOVERNANCE_UPDATE.remove(storage)
}
