use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary, Addr, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use nexus_prism_protocol::{
    cfg_addr, cfg_var,
    common::{div, get_price, get_time, mint, mul, transfer},
};
use prism_protocol::{
    launch_pool::{DistributionStatusResponse, RewardInfoResponse},
    xprism_boost::UserInfo,
};

use crate::{
    error::ContractError,
    replies_id::ReplyId,
    state::{
        load_config, load_state, save_config, save_state, Config, GovernanceUpdateState,
        ReplyContext, State, GOVERNANCE_UPDATE, REPLY_CONTEXT,
    },
};

pub fn update_state(
    deps: DepsMut,
    config: Config,
    nexprism_stakers_reward_ratio: Decimal,
    nyluna_stakers_reward_ratio: Decimal,
    psi_stakers_reward_ratio: Decimal,
    last_calculation_time: Option<u64>,
) -> Result<Response, ContractError> {
    let mut state = load_state(deps.storage)?;

    state.nexprism_stakers_reward_ratio = nexprism_stakers_reward_ratio;
    state.nyluna_stakers_reward_ratio = nyluna_stakers_reward_ratio;
    state.psi_stakers_reward_ratio = psi_stakers_reward_ratio;
    cfg_var!(state, last_calculation_time);

    save_state(deps.storage, &config, &state)?;

    Ok(Response::new().add_attribute("action", "update_state"))
}

#[allow(clippy::too_many_arguments)]
pub fn update_config_by_owner(
    deps: DepsMut,
    mut config: Config,
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
) -> Result<Response, ContractError> {
    cfg_addr!(
        deps,
        config,
        owner,
        prism_launch_pool,
        prism_xprism_boost,
        prism_xprism_pair,
        prism_yluna_pair
    );

    if let Some(rewards_distribution_update_period_secs) = rewards_distribution_update_period_secs {
        config.rewards_distribution_update_period_secs =
            if rewards_distribution_update_period_secs != 0 {
                Some(rewards_distribution_update_period_secs)
            } else {
                None
            };
    }

    cfg_var!(
        config,
        rewards_distribution_update_step,
        min_nexprism_stakers_reward_ratio,
        max_nexprism_stakers_reward_ratio,
        min_nyluna_stakers_reward_ratio,
        max_nyluna_stakers_reward_ratio
    );

    save_config(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[allow(clippy::too_many_arguments)]
pub fn update_config_by_governance(
    deps: DepsMut,
    mut config: Config,
    owner: Option<String>,
    rewards_distribution_update_period_secs: Option<u64>,
    rewards_distribution_update_step: Option<Decimal>,
    min_nexprism_stakers_reward_ratio: Option<Decimal>,
    max_nexprism_stakers_reward_ratio: Option<Decimal>,
    min_nyluna_stakers_reward_ratio: Option<Decimal>,
    max_nyluna_stakers_reward_ratio: Option<Decimal>,
) -> Result<Response, ContractError> {
    cfg_addr!(deps, config, owner);

    if let Some(rewards_distribution_update_period_secs) = rewards_distribution_update_period_secs {
        config.rewards_distribution_update_period_secs =
            if rewards_distribution_update_period_secs != 0 {
                Some(rewards_distribution_update_period_secs)
            } else {
                None
            };
    }

    cfg_var!(
        config,
        rewards_distribution_update_step,
        min_nexprism_stakers_reward_ratio,
        max_nexprism_stakers_reward_ratio,
        min_nyluna_stakers_reward_ratio,
        max_nyluna_stakers_reward_ratio
    );

    save_config(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn deposit_xprism(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    config: Config,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut state = load_state(deps.storage)?;
    state.xprism_amount_total += amount;
    update_rewards_distribution_by_anyone(deps.as_ref(), env, &config, &mut state)?;
    save_state(deps.storage, &config, &state)?;

    Ok(Response::new()
        .add_submessage(mint(
            &config.nexprism_token,
            &Addr::unchecked(sender),
            amount,
        )?)
        .add_submessage(deposit_to_xprism_boost(
            &config.prism_xprism_boost,
            &config.xprism_token,
            amount,
        )?)
        .add_attribute("action", "deposit_xprism")
        .add_attribute("amount", amount))
}

fn deposit_to_xprism_boost(
    xprism_boost: &Addr,
    xprism_token: &Addr,
    amount: Uint128,
) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: xprism_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: xprism_boost.to_string(),
            amount,
            msg: to_binary(&prism_protocol::xprism_boost::Cw20HookMsg::Bond { user: None })?,
        })?,
        funds: vec![],
    }))
}

pub fn deposit_yluna(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    config: Config,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut state = load_state(deps.storage)?;
    state.yluna_amount_total += amount;
    update_rewards_distribution_by_anyone(deps.as_ref(), env, &config, &mut state)?;
    save_state(deps.storage, &config, &state)?;

    Ok(Response::new()
        .add_submessage(mint(
            &config.nyluna_token,
            &Addr::unchecked(sender),
            amount,
        )?)
        .add_submessage(deposit_to_launch_pool(
            &config.prism_launch_pool,
            &config.yluna_token,
            amount,
        )?)
        .add_attribute("action", "deposit_yluna")
        .add_attribute("amount", amount))
}

fn deposit_to_launch_pool(
    launch_pool: &Addr,
    yluna_token: &Addr,
    amount: Uint128,
) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: yluna_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: launch_pool.to_string(),
            amount,
            msg: to_binary(&prism_protocol::launch_pool::Cw20HookMsg::Bond {})?,
        })?,
        funds: vec![],
    }))
}

pub fn withdraw_yluna(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    config: Config,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut state = load_state(deps.storage)?;
    state.yluna_amount_total -= amount;
    update_rewards_distribution_by_anyone(deps.as_ref(), env, &config, &mut state)?;
    save_state(deps.storage, &config, &state)?;

    Ok(Response::new()
        .add_submessage(withdraw_from_launch_pool(
            &config.prism_launch_pool,
            amount,
        )?)
        .add_submessage(transfer(
            &config.yluna_token,
            &Addr::unchecked(sender),
            amount,
        )?)
        .add_attribute("action", "withdraw_yluna")
        .add_attribute("amount", amount))
}

fn withdraw_from_launch_pool(launch_pool: &Addr, amount: Uint128) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: launch_pool.to_string(),
        msg: to_binary(&prism_protocol::launch_pool::ExecuteMsg::Unbond {
            amount: Some(amount),
        })?,
        funds: vec![],
    }))
}

pub fn claim_virtual_rewards(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;

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
        .add_submessage(activate_xprism_boost(&config.prism_xprism_boost)?) // needed only here
        .add_submessage(withdraw_rewards(&config.prism_xprism_boost)?)
        .add_attribute("action", "claim_virtual_rewards"))
}

fn activate_xprism_boost(xprism_boost: &Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: xprism_boost.to_string(),
        msg: to_binary(&prism_protocol::launch_pool::ExecuteMsg::ActivateBoost {})?,
        funds: vec![],
    }))
}

fn withdraw_rewards(xprism_boost: &Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: xprism_boost.to_string(),
            msg: to_binary(&prism_protocol::launch_pool::ExecuteMsg::WithdrawRewards {})?,
            funds: vec![],
        },
        ReplyId::VirtualRewardsClaimed.into(),
    ))
}

pub fn claim_real_rewards(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;

    Ok(Response::new()
        .add_submessage(claim_withdrawn_rewards(&config.prism_xprism_boost)?)
        .add_attribute("action", "claim_real_rewards"))
}

fn claim_withdrawn_rewards(xprism_boost: &Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: xprism_boost.to_string(),
            msg: to_binary(
                &prism_protocol::launch_pool::ExecuteMsg::ClaimWithdrawnRewards {
                    claim_type: prism_protocol::launch_pool::ClaimType::Prism,
                },
            )?,
            funds: vec![],
        },
        ReplyId::RealRewardsClaimed.into(),
    ))
}

pub fn update_rewards_distribution_by_owner(
    deps: DepsMut,
    env: Env,
    config: Config,
) -> Result<Response, ContractError> {
    let mut state = load_state(deps.storage)?;
    state.last_calculation_time = get_time(&env.block);
    let new_state = update_rewards_distribution(deps.as_ref(), env, &config, &state)?;
    save_state(deps.storage, &config, &new_state)?;

    Ok(Response::new()
        .add_attribute("action", "update_rewards_distribution")
        .add_attribute(
            "nexprism_stakers_reward_ratio",
            new_state.nexprism_stakers_reward_ratio.to_string(),
        )
        .add_attribute(
            "nyluna_stakers_reward_ratio",
            new_state.nyluna_stakers_reward_ratio.to_string(),
        )
        .add_attribute(
            "psi_stakers_reward_ratio",
            new_state.psi_stakers_reward_ratio.to_string(),
        ))
}

fn update_rewards_distribution_by_anyone(
    deps: Deps,
    env: Env,
    config: &Config,
    state: &mut State,
) -> Result<(), ContractError> {
    if let Some(period) = config.rewards_distribution_update_period_secs {
        let cur_time = get_time(&env.block);
        if state.last_calculation_time + period < cur_time {
            return Ok(());
        }

        state.last_calculation_time = cur_time;
        *state = update_rewards_distribution(deps, env, config, &state)?;
        return Ok(());
    }

    Ok(())
}

pub fn update_rewards_distribution(
    deps: Deps,
    env: Env,
    config: &Config,
    state: &State,
) -> StdResult<State> {
    let xprism_price = get_price(
        deps,
        &config.prism_xprism_pair,
        &config.xprism_token,
        &config.prism_token,
    )?;
    let yluna_price = get_price(
        deps,
        &config.prism_yluna_pair,
        &config.yluna_token,
        &config.prism_token,
    )?;

    let value = calculate(
        deps,
        env,
        &config.prism_launch_pool,
        &config.prism_xprism_boost,
        yluna_price,
        xprism_price,
    )?;

    let mut new_state = state.clone();
    match value {
        Value::Negative => {
            new_state.nexprism_stakers_reward_ratio = mul(
                new_state.nexprism_stakers_reward_ratio,
                config.rewards_distribution_update_step,
            );
            if new_state.nexprism_stakers_reward_ratio > config.max_nexprism_stakers_reward_ratio {
                return Ok(state.clone());
            }
            new_state.nyluna_stakers_reward_ratio = Decimal::one()
                - new_state.nexprism_stakers_reward_ratio
                - new_state.psi_stakers_reward_ratio;
            if new_state.nyluna_stakers_reward_ratio < config.min_nyluna_stakers_reward_ratio {
                return Ok(state.clone());
            }
        }
        Value::Positive => {
            new_state.nexprism_stakers_reward_ratio = div(
                new_state.nexprism_stakers_reward_ratio,
                config.rewards_distribution_update_step,
            );
            if new_state.nexprism_stakers_reward_ratio < config.min_nexprism_stakers_reward_ratio {
                return Ok(state.clone());
            }
            new_state.nyluna_stakers_reward_ratio = Decimal::one()
                - new_state.nexprism_stakers_reward_ratio
                - new_state.psi_stakers_reward_ratio;
            if new_state.nyluna_stakers_reward_ratio > config.max_nyluna_stakers_reward_ratio {
                return Ok(state.clone());
            }
        }
        Value::Zero => {
            return Ok(state.clone());
        }
    }

    Ok(new_state)
}

enum Value {
    Zero,
    Positive,
    Negative,
}

fn calculate(
    deps: Deps,
    env: Env,
    prism_launch_pool: &Addr,
    prism_xprism_boost: &Addr,
    yluna_price: Decimal,
    xprism_price: Decimal,
) -> Result<Value, ContractError> {
    let addr = env.contract.address;

    let user_info: UserInfo = deps.querier.query_wasm_smart(
        prism_xprism_boost,
        &prism_protocol::xprism_boost::QueryMsg::GetBoost { user: addr.clone() },
    )?;
    let dist_status: DistributionStatusResponse = deps.querier.query_wasm_smart(
        prism_launch_pool,
        &prism_protocol::launch_pool::QueryMsg::DistributionStatus {},
    )?;
    let launch_pool_config: prism_protocol::launch_pool::ConfigResponse =
        deps.querier.query_wasm_smart(
            prism_launch_pool,
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
        std::cmp::Ordering::Greater => Value::Positive,
        std::cmp::Ordering::Less => Value::Negative,
        std::cmp::Ordering::Equal => Value::Zero,
    }
}

pub fn accept_governance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let gov_update = GOVERNANCE_UPDATE.load(deps.storage)?;
    let cur_time = get_time(&env.block);

    if gov_update.wait_approve_until < cur_time {
        return Err(StdError::generic_err("too late to accept governance owning").into());
    }

    if info.sender != gov_update.new_governance {
        return Err(ContractError::Unauthorized {});
    }

    let new_gov_addr = gov_update.new_governance.to_string();

    let mut config = load_config(deps.storage)?;
    config.governance = gov_update.new_governance;
    save_config(deps.storage, &config)?;
    GOVERNANCE_UPDATE.remove(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "update_governance")
        .add_attribute("new_addr", &new_gov_addr))
}

pub fn update_governance(
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
