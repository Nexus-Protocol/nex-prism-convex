use cosmwasm_std::Decimal;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub governance: String,

    pub cw20_token_code_id: u64,
    pub staking_code_id: u64,
    pub autocompounder_code_id: u64,

    pub astroport_factory: String,
    pub nexprism_xprism_amp_coef: u64,

    pub psi_token: String,
    pub prism_token: String,
    pub xprism_token: String,
    pub yluna_token: String,

    pub prism_governance: String,
    pub prism_launch_pool: String,
    pub prism_xprism_boost: String,

    pub prism_xprism_pair: String,
    pub prism_yluna_pair: String,

    pub rewards_distribution_update_period_secs: Option<u64>,
    pub rewards_distribution_update_step: Decimal,

    pub nexprism_stakers_reward_ratio: Decimal,
    pub min_nexprism_stakers_reward_ratio: Decimal,
    pub max_nexprism_stakers_reward_ratio: Decimal,

    pub nyluna_stakers_reward_ratio: Decimal,
    pub min_nyluna_stakers_reward_ratio: Decimal,
    pub max_nyluna_stakers_reward_ratio: Decimal,

    pub psi_stakers_reward_ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    ClaimRealRewards {},
    ClaimVirtualRewards {},
    Owner { msg: OwnerMsg },
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
pub enum OwnerMsg {
    UpdateRewardsDistribution {},
    UpdateState {
        nexprism_stakers_reward_ratio: Decimal,
        nyluna_stakers_reward_ratio: Decimal,
        psi_stakers_reward_ratio: Decimal,
        last_calculation_time: Option<u64>,
    },
    UpdateConfig {
        owner: Option<String>,

        prism_launch_pool: Option<String>,
        prism_xprism_boost: Option<String>,

        prism_xprism_pair: Option<String>,
        prism_yluna_pair: Option<String>,

        rewards_distribution_update_period_secs: Option<u64>,
        rewards_distribution_update_step: Option<Decimal>,

        min_nexprism_stakers_reward_ratio: Option<Decimal>,
        max_nexprism_stakers_reward_ratio: Option<Decimal>,

        min_nyluna_stakers_reward_ratio: Option<Decimal>,
        max_nyluna_stakers_reward_ratio: Option<Decimal>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceMsg {
    UpdateState {
        nexprism_stakers_reward_ratio: Decimal,
        nyluna_stakers_reward_ratio: Decimal,
        psi_stakers_reward_ratio: Decimal,
        last_calculation_time: Option<u64>,
    },
    UpdateConfig {
        owner: Option<String>,

        rewards_distribution_update_period_secs: Option<u64>,
        rewards_distribution_update_step: Option<Decimal>,

        min_nexprism_stakers_reward_ratio: Option<Decimal>,
        max_nexprism_stakers_reward_ratio: Option<Decimal>,

        min_nyluna_stakers_reward_ratio: Option<Decimal>,
        max_nyluna_stakers_reward_ratio: Option<Decimal>,
    },
    UpdateGovernance {
        addr: String,
        seconds_to_wait_for_accept_gov_tx: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State {},
    SimulateUpdateRewardsDistribution {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub governance: String,

    pub xprism_token: String,
    pub nexprism_token: String,
    pub yluna_token: String,
    pub nyluna_token: String,
    pub prism_token: String,

    pub prism_launch_pool: String,
    pub prism_xprism_boost: String,

    pub nexprism_staking: String,
    pub psi_staking: String,
    pub nyluna_staking: String,

    pub nexprism_xprism_pair: String,
    pub prism_xprism_pair: String,
    pub prism_yluna_pair: String,

    pub rewards_distribution_update_period_secs: Option<u64>,
    pub rewards_distribution_update_step: Decimal,

    pub min_nexprism_stakers_reward_ratio: Decimal,
    pub max_nexprism_stakers_reward_ratio: Decimal,
    pub min_nyluna_stakers_reward_ratio: Decimal,
    pub max_nyluna_stakers_reward_ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub nexprism_stakers_reward_ratio: Decimal,
    pub nyluna_stakers_reward_ratio: Decimal,
    pub psi_stakers_reward_ratio: Decimal,
    pub last_calculation_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateRewardsDistributionResponse {
    pub nexprism_stakers_reward_ratio: Decimal,
    pub nyluna_stakers_reward_ratio: Decimal,
    pub psi_stakers_reward_ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
