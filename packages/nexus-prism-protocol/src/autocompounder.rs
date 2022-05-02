use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub compounding_token: String,
    pub reward_token: String,
    pub reward_compound_pair: String,
    pub governance: String,
    pub staking_contract: String,
    pub cw20_token_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Governance { governance_msg: GovernanceMsg },
    AcceptGovernance {},
    Compound {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceMsg {
    UpdateConfig {
        reward_compound_pair: Option<String>,
        staking_contract: Option<String>,
    },
    UpdateGovernanceContract {
        gov_addr: String,
        //how long to wait for 'AcceptGovernance' transaction
        seconds_to_wait_for_accept_gov_tx: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Deposit {},
    Withdraw {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    AutoCompoundingTokenValue { amount: Uint128 },
    CompoundingTokenValue { amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub compounding_token: String,
    pub auto_compounding_token: String,
    pub reward_token: String,
    pub reward_compound_pair: String,
    pub governance: String,
    pub staking_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AutoCompoundingTokenValueResponse {
    pub compounding_token_amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CompoundingTokenValueResponse {
    pub auto_compounding_token_amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
