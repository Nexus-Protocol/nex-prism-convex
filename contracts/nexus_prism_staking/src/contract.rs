use std::convert::TryFrom;

use cosmwasm_std::{
    entry_point, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, Uint128,
};
use cw0::nonpayable;
use cw2::{get_contract_version, set_contract_version};
use nexus_prism_protocol::common::{optional_addr_validate, send};

use crate::commands::{
    accept_governance, claim_rewards, claim_rewards_for_someone, decrease_balance,
    increase_balance, receive_cw20, reward, unbond, update_global_index, update_governance,
};
use crate::replies_id::ReplyId;
use crate::state::{Config, REPLY_CONTEXT};
use crate::{
    commands,
    error::ContractError,
    queries,
    state::{load_config, save_config, save_state, RewardState, State},
};
use nexus_prism_protocol::{
    common::query_token_balance,
    staking::{
        AnyoneMsg, ExecuteMsg, GovernanceMsg, InstantiateMsg, MigrateMsg, QueryMsg,
        RewardOperatorMsg, StakeOperatorMsg,
    },
};

const CONTRACT_NAME: &str = "nexus.protocol:nex-prism-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let config = Config {
        governance: deps.api.addr_validate(&msg.governance)?,
        staking_token: deps.api.addr_validate(&msg.staking_token)?,
        stake_operator: optional_addr_validate(deps.as_ref(), msg.stake_operator)?,
        reward_token: deps.api.addr_validate(&msg.reward_token)?,
        reward_operator: deps.api.addr_validate(&msg.reward_operator)?,
        xprism_token: optional_addr_validate(deps.as_ref(), msg.xprism_token)?,
        prism_governance: optional_addr_validate(deps.as_ref(), msg.prism_governance)?,
        nexprism_xprism_pair: optional_addr_validate(deps.as_ref(), msg.nexprism_xprism_pair)?,
    };
    save_config(deps.storage, &config)?;

    save_state(
        deps.storage,
        &State {
            staking_total_balance: Uint128::zero(),
            virtual_reward_balance: Uint128::zero(),
            virtual_rewards: RewardState {
                global_index: Decimal::zero(),
                prev_balance: Uint128::zero(),
            },
            real_rewards: RewardState {
                global_index: Decimal::zero(),
                prev_balance: Uint128::zero(),
            },
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),

        ExecuteMsg::Anyone { anyone_msg } => match anyone_msg {
            AnyoneMsg::Unbond { amount } => unbond(deps, env, info, amount),
            AnyoneMsg::UpdateGlobalIndex {} => update_global_index(deps, env),
            AnyoneMsg::ClaimRewards { recipient } => claim_rewards(deps, env, info, recipient),
            AnyoneMsg::ClaimRewardsForSomeone { address } => {
                claim_rewards_for_someone(deps, env, address)
            }
            AnyoneMsg::AcceptGovernance {} => accept_governance(deps, env, info),
        },

        ExecuteMsg::StakeOperator { msg } => {
            let config = load_config(deps.storage)?;
            if Some(info.sender) != config.stake_operator {
                return Err(ContractError::Unauthorized);
            }
            match msg {
                StakeOperatorMsg::IncreaseBalance { staker, amount } => {
                    increase_balance(deps, env, &config, staker, amount)
                }
                StakeOperatorMsg::DecreaseBalance { staker, amount } => {
                    decrease_balance(deps, env, &config, staker, amount)
                }
            }
        }

        ExecuteMsg::RewardOperator { msg } => {
            let config = load_config(deps.storage)?;
            if info.sender != config.reward_operator {
                return Err(ContractError::Unauthorized);
            }
            match msg {
                RewardOperatorMsg::Reward { amount } => reward(deps, amount),
            }
        }

        ExecuteMsg::Governance { governance_msg } => {
            let config = load_config(deps.storage)?;
            if info.sender != config.governance {
                return Err(ContractError::Unauthorized);
            }
            match governance_msg {
                GovernanceMsg::UpdateConfig {
                    stake_operator,
                    reward_operator,
                    nexprism_xprism_pair,
                } => commands::update_config(
                    deps,
                    config,
                    stake_operator,
                    reward_operator,
                    nexprism_xprism_pair,
                ),
                GovernanceMsg::UpdateGovernance {
                    gov_addr,
                    seconds_to_wait_for_accept_gov_tx,
                } => update_governance(deps, env, gov_addr, seconds_to_wait_for_accept_gov_tx),
            }
        }
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;

    let reply_id =
        ReplyId::try_from(msg.id).map_err(|_| ContractError::UnknownReplyId { id: msg.id })?;

    match reply_id {
        ReplyId::XPrismTokensMinted => match (config.xprism_token, config.nexprism_xprism_pair) {
            (Some(xprism_token), Some(nexprism_xprism_pair)) => {
                let context = REPLY_CONTEXT.load(deps.storage)?;
                let xprism_balance =
                    query_token_balance(deps.as_ref(), &xprism_token, &env.contract.address);
                Ok(Response::new()
                    .add_submessage(send(
                        &xprism_token,
                        &nexprism_xprism_pair,
                        xprism_balance,
                        &astroport::pair::Cw20HookMsg::Swap {
                            belief_price: None,
                            max_spread: None,
                            to: Some(context.rewards_recipient.to_string()),
                        },
                    )?)
                    .add_attribute("minted_xprism_amount", xprism_balance)
                    .add_attribute("recipient", context.rewards_recipient))
            }
            _ => Err(ContractError::InvalidConfig {}),
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::State {} => to_binary(&queries::query_state(deps)?),
        QueryMsg::Rewards { address } => to_binary(&queries::query_rewards(deps, address)?),
        QueryMsg::Staker { address } => to_binary(&queries::query_staker(deps, env, address)?),
        QueryMsg::GetPotentialRewards {
            potential_rewards_total,
            address,
        } => to_binary(&queries::query_potential_rewards(
            deps,
            env,
            potential_rewards_total,
            address,
        )?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let ver = get_contract_version(deps.storage)?;

    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type"));
    }

    if ver.version.as_str() >= CONTRACT_VERSION {
        return Err(StdError::generic_err("Cannot upgrade from a newer version"));
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}
