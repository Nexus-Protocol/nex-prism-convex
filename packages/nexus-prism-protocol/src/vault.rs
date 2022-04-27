use cosmwasm_std::Decimal;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub governance: String,
    pub psi_token: String,
    pub cw20_token_code_id: u64,
    pub staking_code_id: u64,
    pub astroport_factory: String,
    pub xprism_token: String,
    pub yluna_token: String,
    pub prism_token: String,
    pub prism_launch_pool: String,
    pub prism_xprism_boost: String,
    pub xprism_prism_pair: String,
    pub yluna_prism_pair: String,
    pub rewards_distribution_update_period: Option<u64>,
    pub rewards_distribution_update_step: Decimal,
    pub nexprism_stakers_reward_ratio: Decimal,
    pub yluna_depositors_reward_ratio: Decimal,
    pub psi_stakers_reward_ratio: Decimal,
    pub min_nexprism_stakers_reward_ratio: Decimal,
    pub max_nexprism_stakers_reward_ratio: Decimal,
    pub min_yluna_depositors_reward_ratio: Decimal,
    pub max_yluna_depositors_reward_ratio: Decimal,
}

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    ClaimRealRewards {},
    ClaimVirtualRewards {},
    Governance { msg: GovernanceMsg },
    AcceptGovernance {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Deposit {},
    Withdraw {},
}

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceMsg {
    UpdateConfig {
        owner: Option<String>,
        xprism_token: Option<String>,
        nexprism_token: Option<String>,
        yluna_token: Option<String>,
        prism_token: Option<String>,
        prism_launch_pool: Option<String>,
        prism_xprism_boost: Option<String>,
        nexprism_xprism_staking: Option<String>,
        psi_nexprism_staking: Option<String>,
        yluna_prism_staking: Option<String>,
        xprism_prism_pair: Option<String>,
        yluna_prism_pair: Option<String>,
        rewards_distribution_update_period: Option<u64>,
        rewards_distribution_update_step: Option<Decimal>,
        min_nexprism_stakers_reward_ratio: Option<Decimal>,
        max_nexprism_stakers_reward_ratio: Option<Decimal>,
        min_yluna_depositors_reward_ratio: Option<Decimal>,
        max_yluna_depositors_reward_ratio: Option<Decimal>,
    },
    UpdateGovernanceContract {
        addr: String,
        seconds_to_wait_for_accept_gov_tx: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub governance: String,
    pub xprism_token: String,
    pub nexprism_token: String,
    pub yluna_token: String,
    pub prism_token: String,
    pub prism_launch_pool: String,
    pub prism_xprism_boost: String,
    pub nexprism_xprism_staking: String,
    pub psi_nexprism_staking: String,
    pub yluna_prism_staking: String,
    pub xprism_prism_pair: String,
    pub yluna_prism_pair: String,
    pub rewards_distribution_update_period: Option<u64>,
    pub rewards_distribution_update_step: Decimal,
    pub min_nexprism_stakers_reward_ratio: Decimal,
    pub max_nexprism_stakers_reward_ratio: Decimal,
    pub min_yluna_depositors_reward_ratio: Decimal,
    pub max_yluna_depositors_reward_ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
