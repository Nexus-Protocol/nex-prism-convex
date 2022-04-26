use std::cmp::Ordering;

use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw0::nonpayable;
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use nexus_prism_protocol::common::{div, get_price, mint, mul, query_token_balance, transfer};
use nexus_prism_protocol::vault::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, GovernanceMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use prism_protocol::launch_pool::{DistributionStatusResponse, RewardInfoResponse};
use prism_protocol::xprism_boost::UserInfo;

use crate::error::ContractError;
use crate::state::{
    save_state, Config, GovernanceUpdateState, ReplyContext, State, CONFIG, GOVERNANCE_UPDATE,
    REPLY_CONTEXT, STATE,
};

const CONTRACT_NAME: &str = "nexus.protocol:nex-prism-convex";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let config = Config {
        owner: info.sender,
        governance: deps.api.addr_validate(&msg.governance)?,
        xprism_token: deps.api.addr_validate(&msg.xprism_token)?,
        nexprism_token: deps.api.addr_validate(&msg.nexprism_token)?,
        yluna_token: deps.api.addr_validate(&msg.yluna_token)?,
        nyluna_token: deps.api.addr_validate(&msg.nyluna_token)?,
        prism_token: deps.api.addr_validate(&msg.prism_token)?,
        prism_launch_pool: deps.api.addr_validate(&msg.prism_launch_pool)?,
        prism_xprism_boost: deps.api.addr_validate(&msg.prism_xprism_boost)?,
        nexprism_xprism_staking: deps.api.addr_validate(&msg.nexprism_xprism_staking)?,
        psi_nexprism_staking: deps.api.addr_validate(&msg.psi_nexprism_staking)?,
        yluna_prism_staking: deps.api.addr_validate(&msg.yluna_prism_staking)?,
        xprism_prism_pair: deps.api.addr_validate(&msg.xprism_prism_pair)?,
        yluna_prism_pair: deps.api.addr_validate(&msg.yluna_prism_pair)?,
        rewards_distribution_update_period: msg.rewards_distribution_update_period,
        rewards_distribution_update_step: msg.rewards_distribution_update_step,
        min_nexprism_stakers_reward_ratio: msg.nexprism_stakers_reward_ratio,
        max_nexprism_stakers_reward_ratio: msg.nexprism_stakers_reward_ratio,
        min_yluna_depositors_reward_ratio: msg.yluna_depositors_reward_ratio,
        max_yluna_depositors_reward_ratio: msg.yluna_depositors_reward_ratio,
    };
    CONFIG.save(deps.storage, &config)?;

    save_state(
        deps.storage,
        &config,
        &State {
            nexprism_stakers_reward_ratio: msg.nexprism_stakers_reward_ratio,
            yluna_depositors_reward_ratio: msg.yluna_depositors_reward_ratio,
            psi_stakers_reward_ratio: msg.psi_stakers_reward_ratio,
            last_calculation_time: 0,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::ClaimVirtualRewards {} => execute_claim_virtual_rewards(deps, env, info),
        ExecuteMsg::ClaimRealRewards {} => execute_claim_real_rewards(deps, env, info),
        ExecuteMsg::AcceptGovernance {} => execute_accept_governance(deps, env, info),
        ExecuteMsg::Governance { msg } => {
            nonpayable(&info)?;

            let config = CONFIG.load(deps.storage)?;

            if info.sender != config.governance {
                return Err(ContractError::Unauthorized {});
            }

            match msg {
                GovernanceMsg::UpdateConfig {
                    owner,
                    xprism_token,
                    nexprism_token,
                    yluna_token,
                    prism_token,
                    prism_launch_pool,
                    prism_xprism_boost,
                    nexprism_xprism_staking,
                    psi_nexprism_staking,
                    yluna_prism_staking,
                    xprism_prism_pair,
                    yluna_prism_pair,
                    rewards_distribution_update_period,
                    rewards_distribution_update_step,
                    min_nexprism_stakers_reward_ratio,
                    max_nexprism_stakers_reward_ratio,
                    min_yluna_depositors_reward_ratio,
                    max_yluna_depositors_reward_ratio,
                } => update_config(
                    deps,
                    config,
                    owner,
                    xprism_token,
                    nexprism_token,
                    yluna_token,
                    prism_token,
                    prism_launch_pool,
                    prism_xprism_boost,
                    nexprism_xprism_staking,
                    psi_nexprism_staking,
                    yluna_prism_staking,
                    xprism_prism_pair,
                    yluna_prism_pair,
                    rewards_distribution_update_period,
                    rewards_distribution_update_step,
                    min_nexprism_stakers_reward_ratio,
                    max_nexprism_stakers_reward_ratio,
                    min_yluna_depositors_reward_ratio,
                    max_yluna_depositors_reward_ratio,
                ),
                GovernanceMsg::UpdateGovernanceContract {
                    addr,
                    seconds_to_wait_for_accept_gov_tx,
                } => update_governance_addr(deps, env, addr, seconds_to_wait_for_accept_gov_tx),
            }
        }
    }
}

fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let config = CONFIG.load(deps.storage)?;
    let token = info.sender.clone();

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Deposit {}) if token == config.xprism_token => {
            execute_deposit_xprism(deps, env, info, config, cw20_msg.sender, cw20_msg.amount)
        }
        Ok(Cw20HookMsg::Deposit {}) if token == config.yluna_token => {
            execute_deposit_yluna(deps, env, info, config, cw20_msg.sender, cw20_msg.amount)
        }
        Ok(Cw20HookMsg::Deposit {}) => Err(ContractError::Unauthorized {}),

        Ok(Cw20HookMsg::Withdraw {}) if token == config.nyluna_token => {
            execute_withdraw_yluna(deps, env, info, config, cw20_msg.sender, cw20_msg.amount)
        }
        Ok(Cw20HookMsg::Withdraw {}) => Err(ContractError::Unauthorized {}),

        Err(err) => Err(ContractError::Std(err)),
    }
}

fn execute_deposit_xprism(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    config: Config,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    update_rewards_distribution(deps, env, &config)?;

    Ok(Response::new()
        .add_submessage(mint(config.nexprism_token.to_string(), sender, amount)?)
        .add_submessage(deposit_to_xprism_boost(
            config.prism_xprism_boost.to_string(),
            config.xprism_token.to_string(),
            amount,
        )?)
        .add_attribute("action", "deposit_xprism")
        .add_attribute("amount", amount))
}

fn update_rewards_distribution(
    deps: DepsMut,
    env: Env,
    config: &Config,
) -> Result<(), ContractError> {
    if let Some(period) = config.rewards_distribution_update_period {
        let mut state = STATE.load(deps.storage)?;

        let cur_time = env.block.time.seconds();
        if state.last_calculation_time + period < cur_time {
            return Ok(());
        }

        state.last_calculation_time = cur_time;
        save_state(deps.storage, config, &state)?;

        let xprism_price = get_price(
            deps.as_ref(),
            &config.xprism_prism_pair,
            &config.xprism_token,
            &config.prism_token,
        )?;
        let yluna_price = get_price(
            deps.as_ref(),
            &config.yluna_prism_pair,
            &config.yluna_token,
            &config.prism_token,
        )?;

        let value = calculate(
            deps.as_ref(),
            env,
            config.prism_launch_pool.clone(),
            config.prism_xprism_boost.clone(),
            yluna_price,
            xprism_price,
        )?;
        match value {
            Value::Negative => {
                state.nexprism_stakers_reward_ratio = mul(
                    state.nexprism_stakers_reward_ratio,
                    config.rewards_distribution_update_step,
                );
                if state.nexprism_stakers_reward_ratio > config.max_nexprism_stakers_reward_ratio {
                    return Ok(());
                }
                if state.yluna_depositors_reward_ratio < config.rewards_distribution_update_step {
                    return Ok(());
                }
                state.yluna_depositors_reward_ratio = div(
                    state.yluna_depositors_reward_ratio,
                    config.rewards_distribution_update_step,
                );
                if state.yluna_depositors_reward_ratio < config.min_yluna_depositors_reward_ratio {
                    return Ok(());
                }
            }
            Value::Positive => {
                state.yluna_depositors_reward_ratio = mul(
                    state.yluna_depositors_reward_ratio,
                    config.rewards_distribution_update_step,
                );
                if state.yluna_depositors_reward_ratio > config.max_yluna_depositors_reward_ratio {
                    return Ok(());
                }
                if state.nexprism_stakers_reward_ratio < config.rewards_distribution_update_step {
                    return Ok(());
                }
                state.nexprism_stakers_reward_ratio = div(
                    state.nexprism_stakers_reward_ratio,
                    config.rewards_distribution_update_step,
                );
                if state.nexprism_stakers_reward_ratio < config.min_nexprism_stakers_reward_ratio {
                    return Ok(());
                }
            }
            Value::Zero => {
                return Ok(());
            }
        };
        save_state(deps.storage, config, &state)?;
    }

    Ok(())
}

fn deposit_to_xprism_boost(
    addr: String,
    xprism_token: String,
    amount: Uint128,
) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: xprism_token,
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: addr,
            amount,
            msg: to_binary(&prism_protocol::xprism_boost::Cw20HookMsg::Bond { user: None })?,
        })?,
        funds: vec![],
    }))
}

fn execute_deposit_yluna(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    config: Config,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    update_rewards_distribution(deps, env, &config)?;

    Ok(Response::new()
        .add_submessage(mint(config.nyluna_token.to_string(), sender, amount)?)
        .add_submessage(deposit_to_launch_pool(
            config.prism_launch_pool.to_string(),
            config.yluna_token.to_string(),
            amount,
        )?)
        .add_attribute("action", "deposit_yluna")
        .add_attribute("amount", amount))
}

fn deposit_to_launch_pool(addr: String, yluna_token: String, amount: Uint128) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: yluna_token,
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: addr,
            amount,
            msg: to_binary(&prism_protocol::launch_pool::Cw20HookMsg::Bond {})?,
        })?,
        funds: vec![],
    }))
}

/*
fn notify_staking_increase(addr: String, staker: String, amount: Uint128) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: addr,
        msg: to_binary(&nexus_prism_protocol::staking::ExecuteMsg::Owner {
            msg: nexus_prism_protocol::staking::OwnerMsg::IncreaseBalance {
                address: staker,
                amount,
            },
        })?,
        funds: vec![],
    }))
}
*/

fn execute_withdraw_yluna(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    config: Config,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    update_rewards_distribution(deps, env, &config)?;

    Ok(Response::new()
        .add_submessage(withdraw_from_launch_pool(
            config.prism_launch_pool.to_string(),
            amount,
        )?)
        .add_submessage(transfer(config.yluna_token.to_string(), sender, amount)?)
        .add_attribute("action", "withdraw_yluna")
        .add_attribute("amount", amount))
}

fn withdraw_from_launch_pool(addr: String, amount: Uint128) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: addr,
        msg: to_binary(&prism_protocol::launch_pool::ExecuteMsg::Unbond {
            amount: Some(amount),
        })?,
        funds: vec![],
    }))
}

/*
fn notify_staking_decrease(addr: String, staker: String, amount: Uint128) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: addr,
        msg: to_binary(&nexus_prism_protocol::staking::ExecuteMsg::Owner {
            msg: nexus_prism_protocol::staking::OwnerMsg::DecreaseBalance {
                address: staker,
                amount,
            },
        })?,
        funds: vec![],
    }))
}
*/

fn execute_claim_virtual_rewards(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let xprism_boost = config.prism_xprism_boost;

    let reward_info: RewardInfoResponse = deps.querier.query_wasm_smart(
        config.prism_launch_pool,
        &prism_protocol::launch_pool::QueryMsg::RewardInfo {
            staker_addr: env.contract.address.to_string(),
        },
    )?;

    REPLY_CONTEXT.save(
        deps.storage,
        &ReplyContext {
            reward_balance: reward_info.pending_reward,
        },
    )?;

    Ok(Response::new()
        .add_submessage(activate_boost(&xprism_boost)?) // needed only here
        .add_submessage(withdraw_rewards(&xprism_boost)?))
}

fn execute_claim_real_rewards(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let xprism_boost = config.prism_xprism_boost;

    Ok(Response::new().add_submessage(claim_withdrawn_rewards(&xprism_boost)?))
}

fn activate_boost(addr: &Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_binary(&prism_protocol::launch_pool::ExecuteMsg::ActivateBoost {})?,
        funds: vec![],
    }))
}

pub const VIRTUAL_CLAIM_REPLY_ID: u64 = 1;

fn withdraw_rewards(addr: &Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: addr.to_string(),
            msg: to_binary(&prism_protocol::launch_pool::ExecuteMsg::WithdrawRewards {})?,
            funds: vec![],
        },
        VIRTUAL_CLAIM_REPLY_ID,
    ))
}

pub const REAL_CLAIM_REPLY_ID: u64 = 2;

fn claim_withdrawn_rewards(addr: &Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: addr.to_string(),
            msg: to_binary(
                &prism_protocol::launch_pool::ExecuteMsg::ClaimWithdrawnRewards {
                    claim_type: prism_protocol::launch_pool::ClaimType::Prism,
                },
            )?,
            funds: vec![],
        },
        REAL_CLAIM_REPLY_ID,
    ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    match msg.id {
        VIRTUAL_CLAIM_REPLY_ID => {
            let reply_context = REPLY_CONTEXT.load(deps.storage)?;
            let reward_info: RewardInfoResponse = deps.querier.query_wasm_smart(
                config.prism_launch_pool,
                &prism_protocol::launch_pool::QueryMsg::RewardInfo {
                    staker_addr: env.contract.address.to_string(),
                },
            )?;
            let claimed_rewards = reward_info.pending_reward - reply_context.reward_balance;
            let nexprism_stakers_rewards = claimed_rewards * state.nexprism_stakers_reward_ratio;
            let yluna_depositors_rewards = claimed_rewards * state.yluna_depositors_reward_ratio;
            let psi_stakers_rewards = claimed_rewards * state.psi_stakers_reward_ratio;

            Ok(Response::new()
                .add_submessage(SubMsg::new(WasmMsg::Execute {
                    contract_addr: config.nexprism_xprism_staking.to_string(),
                    msg: to_binary(&nexus_prism_protocol::staking::RewarderMsg::Reward {
                        amount: nexprism_stakers_rewards,
                    })?,
                    funds: vec![],
                }))
                .add_submessage(SubMsg::new(WasmMsg::Execute {
                    contract_addr: config.yluna_prism_staking.to_string(),
                    msg: to_binary(&nexus_prism_protocol::staking::RewarderMsg::Reward {
                        amount: yluna_depositors_rewards,
                    })?,
                    funds: vec![],
                }))
                .add_submessage(SubMsg::new(WasmMsg::Execute {
                    contract_addr: config.psi_nexprism_staking.to_string(),
                    msg: to_binary(&nexus_prism_protocol::staking::RewarderMsg::Reward {
                        amount: psi_stakers_rewards,
                    })?,
                    funds: vec![],
                }))
                .add_attribute("action", "claim_virtual_rewards")
                .add_attribute("nexprism_stakers_rewards", nexprism_stakers_rewards)
                .add_attribute("yluna_depositors_rewards", yluna_depositors_rewards)
                .add_attribute("psi_stakers_rewards", psi_stakers_rewards))
        }

        REAL_CLAIM_REPLY_ID => {
            let claimed_rewards =
                query_token_balance(deps.as_ref(), &config.prism_token, &env.contract.address);
            let nexprism_stakers_rewards = claimed_rewards * state.nexprism_stakers_reward_ratio;
            let yluna_depositors_rewards = claimed_rewards * state.yluna_depositors_reward_ratio;
            let psi_stakers_rewards = claimed_rewards * state.psi_stakers_reward_ratio;

            Ok(Response::new()
                .add_submessage(transfer(
                    config.xprism_token.to_string(),
                    config.nexprism_xprism_staking.to_string(),
                    nexprism_stakers_rewards,
                )?)
                .add_submessage(transfer(
                    config.prism_token.to_string(),
                    config.yluna_prism_staking.to_string(),
                    yluna_depositors_rewards,
                )?)
                .add_submessage(transfer(
                    config.nexprism_token.to_string(),
                    config.psi_nexprism_staking.to_string(),
                    psi_stakers_rewards,
                )?)
                .add_attribute("action", "claim_real_rewards")
                .add_attribute("nexprism_stakers_rewards", nexprism_stakers_rewards)
                .add_attribute("yluna_depositors_rewards", yluna_depositors_rewards)
                .add_attribute("psi_stakers_rewards", psi_stakers_rewards))
        }

        _ => Err(ContractError::UnknownReply { id: msg.id }),
    }
}

enum Value {
    Zero,
    Positive,
    Negative,
}

fn calculate(
    deps: Deps,
    env: Env,
    prism_launch_pool: Addr,
    prism_xprism_boost: Addr,
    yluna_price: Decimal,
    xprism_price: Decimal,
) -> Result<Value, ContractError> {
    let addr = env.contract.address;

    let user_info: UserInfo = deps.querier.query_wasm_smart(
        prism_xprism_boost,
        &prism_protocol::xprism_boost::QueryMsg::GetBoost { user: addr.clone() },
    )?;
    let dist_status: DistributionStatusResponse = deps.querier.query_wasm_smart(
        prism_launch_pool.clone(),
        &prism_protocol::launch_pool::QueryMsg::DistributionStatus {},
    )?;
    let launch_pool_config: prism_protocol::launch_pool::ConfigResponse =
        deps.querier.query_wasm_smart(
            prism_launch_pool.clone(),
            &prism_protocol::launch_pool::QueryMsg::Config {},
        )?;
    let reward_info: RewardInfoResponse = deps.querier.query_wasm_smart(
        prism_launch_pool,
        &prism_protocol::launch_pool::QueryMsg::RewardInfo {
            staker_addr: addr.to_string(),
        },
    )?;

    Ok(calculate_inner(
        launch_pool_config.base_pool_ratio.into(),
        user_info.amt_bonded.into(),
        dist_status.base.total_weight.into(),
        reward_info.bond_amount.into(),
        dist_status.boost.total_weight.into(),
        reward_info.boost_weight.into(),
        reward_info.active_boost.into(),
        yluna_price.into(),
        xprism_price.into(),
    ))
}

// TODO: check gas usage
#[allow(clippy::too_many_arguments)]
fn calculate_inner(
    base_ratio: Decimal256,
    xprism: Uint256,
    yluna_total: Uint256,
    yluna: Uint256,
    weight_total: Uint256,
    weight: Uint256,
    ampl: Uint256,
    yluna_price: Decimal256,
    xprism_price: Decimal256,
) -> Value {
    let a = base_ratio * Decimal256::from_ratio(yluna_total - yluna, yluna_total * yluna_total);
    let b = Decimal256::from_uint256(weight_total - weight);
    let c = b + Decimal::from_ratio(ampl * yluna, 1u64).sqrt().into();
    let d = (Decimal256::one() - base_ratio)
        * b
        * Decimal256::from(Decimal::from_ratio(ampl, yluna).sqrt())
        / (Decimal256::from_uint256(2u64) * c * c);
    let e = d * Decimal256::from_ratio(yluna, xprism) * yluna_price / xprism_price;

    match (a + d).cmp(&e) {
        Ordering::Greater => Value::Positive,
        Ordering::Less => Value::Negative,
        Ordering::Equal => Value::Zero,
    }
}

fn execute_accept_governance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let gov_update = GOVERNANCE_UPDATE.load(deps.storage)?;
    let cur_time = env.block.time.seconds();

    if gov_update.wait_approve_until < cur_time {
        return Err(StdError::generic_err("too late to accept governance owning").into());
    }

    if info.sender != gov_update.new_governance {
        return Err(ContractError::Unauthorized {});
    }

    let new_gov_addr = gov_update.new_governance.to_string();

    let mut config = CONFIG.load(deps.storage)?;
    config.governance = gov_update.new_governance;
    CONFIG.save(deps.storage, &config)?;
    GOVERNANCE_UPDATE.remove(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "change_governance_contract")
        .add_attribute("new_addr", &new_gov_addr))
}

#[allow(clippy::too_many_arguments)]
fn update_config(
    deps: DepsMut,
    mut config: Config,
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
) -> Result<Response, ContractError> {
    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(xprism_token) = xprism_token {
        config.xprism_token = deps.api.addr_validate(&xprism_token)?;
    }

    if let Some(nexprism_token) = nexprism_token {
        config.nexprism_token = deps.api.addr_validate(&nexprism_token)?;
    }

    if let Some(yluna_token) = yluna_token {
        config.yluna_token = deps.api.addr_validate(&yluna_token)?;
    }

    if let Some(prism_token) = prism_token {
        config.prism_token = deps.api.addr_validate(&prism_token)?;
    }

    if let Some(prism_launch_pool) = prism_launch_pool {
        config.prism_launch_pool = deps.api.addr_validate(&prism_launch_pool)?;
    }

    if let Some(prism_xprism_boost) = prism_xprism_boost {
        config.prism_xprism_boost = deps.api.addr_validate(&prism_xprism_boost)?;
    }

    if let Some(nexprism_xprism_staking) = nexprism_xprism_staking {
        config.nexprism_xprism_staking = deps.api.addr_validate(&nexprism_xprism_staking)?;
    }

    if let Some(psi_nexprism_staking) = psi_nexprism_staking {
        config.psi_nexprism_staking = deps.api.addr_validate(&psi_nexprism_staking)?;
    }

    if let Some(yluna_prism_staking) = yluna_prism_staking {
        config.yluna_prism_staking = deps.api.addr_validate(&yluna_prism_staking)?;
    }

    if let Some(xprism_prism_pair) = xprism_prism_pair {
        config.xprism_prism_pair = deps.api.addr_validate(&xprism_prism_pair)?;
    }

    if let Some(yluna_prism_pair) = yluna_prism_pair {
        config.yluna_prism_pair = deps.api.addr_validate(&yluna_prism_pair)?;
    }

    if let Some(rewards_distribution_update_period) = rewards_distribution_update_period {
        config.rewards_distribution_update_period = if rewards_distribution_update_period != 0 {
            Some(rewards_distribution_update_period)
        } else {
            None
        };
    }

    if let Some(rewards_distribution_update_step) = rewards_distribution_update_step {
        config.rewards_distribution_update_step = rewards_distribution_update_step;
    }

    if let Some(min_nexprism_stakers_reward_ratio) = min_nexprism_stakers_reward_ratio {
        config.min_nexprism_stakers_reward_ratio = min_nexprism_stakers_reward_ratio;
    }

    if let Some(max_nexprism_stakers_reward_ratio) = max_nexprism_stakers_reward_ratio {
        config.max_nexprism_stakers_reward_ratio = max_nexprism_stakers_reward_ratio;
    }

    if let Some(min_yluna_depositors_reward_ratio) = min_yluna_depositors_reward_ratio {
        config.min_yluna_depositors_reward_ratio = min_yluna_depositors_reward_ratio;
    }

    if let Some(max_yluna_depositors_reward_ratio) = max_yluna_depositors_reward_ratio {
        config.max_yluna_depositors_reward_ratio = max_yluna_depositors_reward_ratio;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn update_governance_addr(
    deps: DepsMut,
    env: Env,
    addr: String,
    seconds_to_wait_for_accept_gov_tx: u64,
) -> Result<Response, ContractError> {
    let cur_time = env.block.time.seconds();
    let gov_update = GovernanceUpdateState {
        new_governance: deps.api.addr_validate(&addr)?,
        wait_approve_until: cur_time + seconds_to_wait_for_accept_gov_tx,
    };
    GOVERNANCE_UPDATE.save(deps.storage, &gov_update)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        governance: config.governance.to_string(),
        xprism_token: config.xprism_token.to_string(),
        nexprism_token: config.nexprism_token.to_string(),
        yluna_token: config.yluna_token.to_string(),
        prism_token: config.prism_token.to_string(),
        prism_launch_pool: config.prism_launch_pool.to_string(),
        prism_xprism_boost: config.prism_xprism_boost.to_string(),
        nexprism_xprism_staking: config.nexprism_xprism_staking.to_string(),
        psi_nexprism_staking: config.psi_nexprism_staking.to_string(),
        yluna_prism_staking: config.yluna_prism_staking.to_string(),
        xprism_prism_pair: config.xprism_prism_pair.to_string(),
        yluna_prism_pair: config.yluna_prism_pair.to_string(),
        rewards_distribution_update_period: config.rewards_distribution_update_period,
        rewards_distribution_update_step: config.rewards_distribution_update_step,
        min_nexprism_stakers_reward_ratio: config.min_nexprism_stakers_reward_ratio,
        max_nexprism_stakers_reward_ratio: config.max_nexprism_stakers_reward_ratio,
        min_yluna_depositors_reward_ratio: config.min_yluna_depositors_reward_ratio,
        max_yluna_depositors_reward_ratio: config.max_yluna_depositors_reward_ratio,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = get_contract_version(deps.storage)?;

    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }

    if ver.version.as_str() >= CONTRACT_VERSION {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}
