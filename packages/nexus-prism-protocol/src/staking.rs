use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub governance: String,
    pub staking_token: String,
    pub stake_operator: Option<String>,
    pub reward_token: String,
    pub reward_operator: String,
    pub xprism_token: Option<String>,
    pub prism_governance: Option<String>,
    pub nexprism_xprism_pair: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Anyone { anyone_msg: AnyoneMsg },
    StakeOperator { msg: StakeOperatorMsg },
    RewardOperator { msg: RewardOperatorMsg },
    Governance { governance_msg: GovernanceMsg },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Bond {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnyoneMsg {
    Unbond { amount: Uint128 },
    UpdateGlobalIndex {},
    ClaimRewards { recipient: Option<String> },
    //Claim rewards for some address, rewards will be sent to it, not to sender!
    ClaimRewardsForSomeone { address: String },
    AcceptGovernance {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RewardOperatorMsg {
    Reward { amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceMsg {
    UpdateConfig {
        stake_operator: Option<String>,
        reward_operator: Option<String>,
        nexprism_xprism_pair: Option<String>,
    },
    UpdateGovernance {
        gov_addr: String,
        seconds_to_wait_for_accept_gov_tx: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StakeOperatorMsg {
    IncreaseBalance { staker: String, amount: Uint128 },
    DecreaseBalance { staker: String, amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State {},
    Rewards { address: String },
    Staker { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub governance: String,
    pub staking_token: String,
    pub stake_operator: Option<String>,
    pub reward_token: String,
    pub reward_operator: String,
    pub xprism_token: Option<String>,
    pub prism_governance: Option<String>,
    pub nexprism_xprism_pair: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub staking_total_balance: Uint128,
    pub virtual_reward_balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardsResponse {
    pub virtual_rewards: Uint128,
    pub real_rewards: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakerResponse {
    pub address: String,
    pub balance: Uint128,
    pub virtual_pending_rewards: Decimal,
    pub real_pending_rewards: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

// ================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StakeOperatorQueryMsg {
    State {},
    Staker { address: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct StakeOperatorStateResponse {
    pub total_deposit: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct StakeOperatorStakerResponse {
    pub balance: Uint128,
}
