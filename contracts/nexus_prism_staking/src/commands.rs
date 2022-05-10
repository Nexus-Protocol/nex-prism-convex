use std::cmp::min;

use cosmwasm_std::{
    from_binary, Addr, BlockInfo, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, SubMsg, Uint128,
};
use nexus_prism_protocol::{
    common::{query_token_balance, send, send_wasm_msg, sum, transfer},
    staking::{
        Cw20HookMsg, StakeOperatorQueryMsg, StakeOperatorStakerResponse, StakeOperatorStateResponse,
    },
};

use crate::{
    error::ContractError,
    replies_id::ReplyId,
    state::{
        load_config, load_gov_update, load_staker, load_state, remove_gov_update, save_config,
        save_gov_update, save_state, Config, GovernanceUpdateState, ReplyContext, RewardState,
        Staker, State, REPLY_CONTEXT,
    },
    utils::{substract_into_decimal, sum_decimals_and_split_result_to_uint_and_decimal},
};
use crate::{state::save_staker, utils::calculate_decimal_rewards};
use cw20::Cw20ReceiveMsg;

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;

    if config.with_stake_operator() || info.sender != config.staking_token {
        return Err(ContractError::Unauthorized);
    }

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Bond {}) => {
            increase_balance(deps, env, &config, cw20_msg.sender, cw20_msg.amount)
        }
        Err(err) => Err(ContractError::Std(err)),
    }
}

pub fn unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;

    if config.with_stake_operator() {
        return Err(ContractError::Unauthorized);
    }

    Ok(
        decrease_balance(deps, env, &config, info.sender.to_string(), amount)?
            .add_submessage(transfer(&config.staking_token, &info.sender, amount)?),
    )
}

pub fn update_config(
    deps: DepsMut,
    mut config: Config,
    stake_operator: Option<String>,
    reward_operator: Option<String>,
    nexprism_xprism_pair: Option<String>,
) -> Result<Response, ContractError> {
    if let Some(stake_operator) = stake_operator {
        config.stake_operator = Some(deps.api.addr_validate(&stake_operator)?);
    }

    if let Some(reward_operator) = reward_operator {
        config.reward_operator = deps.api.addr_validate(&reward_operator)?;
    }

    if let Some(nexprism_xprism_pair) = nexprism_xprism_pair {
        config.nexprism_xprism_pair = Some(deps.api.addr_validate(&nexprism_xprism_pair)?);
    }

    save_config(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn update_global_index(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut state = load_state(deps.storage)?;
    let config = load_config(deps.storage)?;
    state.staking_total_balance =
        get_staking_total_balance(deps.as_ref(), config.stake_operator.clone(), &state)?;

    let resp = Response::new().add_attribute("action", "update_global_index");

    if state.staking_total_balance.is_zero() {
        return Ok(resp);
    }

    let virtual_claimed_rewards = calculate_global_index(
        state.virtual_reward_balance,
        state.staking_total_balance,
        &mut state.virtual_rewards,
    )?;
    let real_claimed_rewards = calculate_global_index(
        query_token_balance(deps.as_ref(), &config.reward_token, &env.contract.address),
        state.staking_total_balance,
        &mut state.real_rewards,
    )?;

    if virtual_claimed_rewards.is_zero() && real_claimed_rewards.is_zero() {
        return Ok(resp);
    }

    save_state(deps.storage, &state)?;

    Ok(resp
        .add_attribute("real_claimed_rewards", real_claimed_rewards)
        .add_attribute("virtual_claimed_rewards", virtual_claimed_rewards))
}

pub fn update_governance(
    deps: DepsMut,
    env: Env,
    gov_addr: String,
    seconds_to_wait_for_accept_gov_tx: u64,
) -> Result<Response, ContractError> {
    let current_time = get_time(&env.block);
    let gov_update = GovernanceUpdateState {
        new_governance_contract_addr: deps.api.addr_validate(&gov_addr)?,
        wait_approve_until: current_time + seconds_to_wait_for_accept_gov_tx,
    };
    save_gov_update(deps.storage, &gov_update)?;
    Ok(Response::new().add_attribute("action", "update_governance_addr"))
}

pub fn accept_governance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let gov_update = load_gov_update(deps.storage)?;
    let current_time = get_time(&env.block);

    if gov_update.wait_approve_until < current_time {
        return Err(StdError::generic_err("too late to accept governance owning").into());
    }

    if info.sender != gov_update.new_governance_contract_addr {
        return Err(ContractError::Unauthorized);
    }

    let new_gov_add_str = gov_update.new_governance_contract_addr.to_string();

    let mut config = load_config(deps.storage)?;
    config.governance = gov_update.new_governance_contract_addr;
    save_config(deps.storage, &config)?;
    remove_gov_update(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "change_governance_contract")
        .add_attribute("new_address", &new_gov_add_str))
}

pub fn calculate_global_index(
    reward_token_balance: Uint128,
    staking_token_balance_total: Uint128,
    reward_state: &mut RewardState,
) -> Result<Uint128, ContractError> {
    let claimed_rewards = reward_token_balance - reward_state.prev_balance;

    if staking_token_balance_total.is_zero() {
        return Ok(claimed_rewards);
    }

    reward_state.prev_balance = reward_token_balance;

    reward_state.global_index = sum(
        reward_state.global_index,
        Decimal::from_ratio(claimed_rewards, staking_token_balance_total),
    );

    Ok(claimed_rewards)
}

pub fn claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let staker = &info.sender;
    match recipient {
        Some(recipient) => {
            let recipient_addr = deps.api.addr_validate(&recipient)?;
            claim_rewards_logic(deps, env, staker, &recipient_addr)
        }
        None => claim_rewards_logic(deps, env, staker, staker),
    }
}

pub fn claim_rewards_for_someone(
    deps: DepsMut,
    env: Env,
    recipient: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&recipient)?;
    claim_rewards_logic(deps, env, &addr, &addr)
}

fn claim_rewards_logic(
    deps: DepsMut,
    env: Env,
    staker_addr: &Addr,
    recipient: &Addr,
) -> Result<Response, ContractError> {
    let mut staker: Staker = load_staker(deps.storage, staker_addr)?;
    let mut state: State = load_state(deps.storage)?;
    let config: Config = load_config(deps.storage)?;

    state.staking_total_balance =
        get_staking_total_balance(deps.as_ref(), config.stake_operator.clone(), &state)?;
    staker.balance =
        get_staker_balance(deps.as_ref(), config.stake_operator, &staker, staker_addr)?;

    calculate_global_index(
        state.virtual_reward_balance,
        state.staking_total_balance,
        &mut state.virtual_rewards,
    )?;
    calculate_global_index(
        query_token_balance(deps.as_ref(), &config.reward_token, &env.contract.address),
        state.staking_total_balance,
        &mut state.real_rewards,
    )?;

    let real_reward_with_decimals = calculate_decimal_rewards(
        state.real_rewards.global_index,
        staker.real_index,
        staker.balance,
    )?;
    let virtual_reward_with_decimals = calculate_decimal_rewards(
        state.virtual_rewards.global_index,
        staker.virtual_index,
        staker.balance,
    )?;

    let (real_rewards, real_decimals) = sum_decimals_and_split_result_to_uint_and_decimal(
        real_reward_with_decimals,
        staker.real_pending_rewards,
    )?;

    let (virtual_rewards, virtual_decimals) = sum_decimals_and_split_result_to_uint_and_decimal(
        virtual_reward_with_decimals,
        staker.virtual_pending_rewards,
    )?;

    let rewards = min(real_rewards, virtual_rewards);
    if rewards.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    state.real_rewards.prev_balance -= rewards;
    state.virtual_rewards.prev_balance -= rewards;
    state.virtual_reward_balance -= rewards;
    save_state(deps.storage, &state)?;

    staker.real_pending_rewards = substract_into_decimal(real_rewards, rewards) + real_decimals;
    staker.virtual_pending_rewards =
        substract_into_decimal(virtual_rewards, rewards) + virtual_decimals;

    staker.real_index = state.real_rewards.global_index;
    staker.virtual_index = state.virtual_rewards.global_index;

    save_staker(deps.storage, staker_addr, &staker)?;

    let resp = Response::new()
        .add_attribute("action", "claim_reward")
        .add_attribute("staker", staker_addr)
        .add_attribute("recipient", recipient)
        .add_attribute("rewards", rewards);

    match (
        config.prism_governance,
        config.xprism_token,
        config.nexprism_xprism_pair,
    ) {
        (None, None, None) => {
            Ok(resp.add_submessage(transfer(&config.reward_token, recipient, rewards)?))
        }
        (Some(prism_gov), None, None) => Ok(resp.add_submessage(prism_xprism_swap(
            &config.reward_token,
            &prism_gov,
            rewards,
            recipient,
        )?)),
        (Some(prism_gov), Some(_), Some(_)) => {
            REPLY_CONTEXT.save(
                deps.storage,
                &ReplyContext {
                    rewards_recipient: recipient.clone(),
                },
            )?;
            Ok(resp.add_submessage(prism_xprism_swap_and_reply(
                &config.reward_token,
                &prism_gov,
                rewards,
            )?))
        }
        _ => Err(ContractError::InvalidConfig {}),
    }
}

pub fn increase_balance(
    deps: DepsMut,
    env: Env,
    config: &Config,
    address: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let address = deps.api.addr_validate(&address)?;

    let mut state: State = load_state(deps.storage)?;
    let mut staker: Staker = load_staker(deps.storage, &address)?;

    state.staking_total_balance =
        get_staking_total_balance(deps.as_ref(), config.stake_operator.clone(), &state)?;
    staker.balance = get_staker_balance(
        deps.as_ref(),
        config.stake_operator.clone(),
        &staker,
        &address,
    )?;

    //cause if so - we already have updated balance from StakeOperator
    if config.with_stake_operator() {
        staker.balance -= amount;
        state.staking_total_balance -= amount;
    }

    let real_rewards = calculate_decimal_rewards(
        state.real_rewards.global_index,
        staker.real_index,
        staker.balance,
    )?;

    staker.real_index = state.real_rewards.global_index;
    staker.real_pending_rewards = sum(real_rewards, staker.real_pending_rewards);

    let virtual_rewards = calculate_decimal_rewards(
        state.virtual_rewards.global_index,
        staker.virtual_index,
        staker.balance,
    )?;

    staker.virtual_index = state.virtual_rewards.global_index;
    staker.virtual_pending_rewards = sum(virtual_rewards, staker.virtual_pending_rewards);

    staker.balance += amount;
    state.staking_total_balance += amount;

    calculate_global_index(
        state.virtual_reward_balance,
        state.staking_total_balance,
        &mut state.virtual_rewards,
    )?;
    calculate_global_index(
        query_token_balance(deps.as_ref(), &config.reward_token, &env.contract.address),
        state.staking_total_balance,
        &mut state.real_rewards,
    )?;
    save_staker(deps.storage, &address, &staker)?;
    save_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "increase_balance")
        .add_attribute("staker", address)
        .add_attribute("amount", amount))
}

pub fn decrease_balance(
    deps: DepsMut,
    env: Env,
    config: &Config,
    address: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let address = deps.api.addr_validate(&address)?;

    let mut state: State = load_state(deps.storage)?;
    let mut staker: Staker = load_staker(deps.storage, &address)?;

    state.staking_total_balance =
        get_staking_total_balance(deps.as_ref(), config.stake_operator.clone(), &state)?;
    staker.balance = get_staker_balance(
        deps.as_ref(),
        config.stake_operator.clone(),
        &staker,
        &address,
    )?;

    //cause if so - we already have updated balance from StakeOperator
    if config.with_stake_operator() {
        staker.balance += amount;
        state.staking_total_balance += amount;
    }

    if !config.with_stake_operator() && staker.balance < amount {
        return Err(ContractError::NotEnoughTokens {
            name: config.staking_token.to_string(),
            value: staker.balance,
            required: amount,
        });
    }

    calculate_global_index(
        state.virtual_reward_balance,
        state.staking_total_balance,
        &mut state.virtual_rewards,
    )?;
    calculate_global_index(
        query_token_balance(deps.as_ref(), &config.reward_token, &env.contract.address),
        state.staking_total_balance,
        &mut state.real_rewards,
    )?;

    let real_rewards = calculate_decimal_rewards(
        state.real_rewards.global_index,
        staker.real_index,
        staker.balance,
    )?;
    staker.real_index = state.real_rewards.global_index;
    staker.real_pending_rewards = sum(real_rewards, staker.real_pending_rewards);

    let virtual_rewards = calculate_decimal_rewards(
        state.virtual_rewards.global_index,
        staker.virtual_index,
        staker.balance,
    )?;
    staker.virtual_index = state.virtual_rewards.global_index;
    staker.virtual_pending_rewards = sum(virtual_rewards, staker.virtual_pending_rewards);

    staker.balance -= amount;
    state.staking_total_balance -= amount;

    save_staker(deps.storage, &address, &staker)?;
    save_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "decrease_balance")
        .add_attribute("staker", address)
        .add_attribute("amount", amount))
}

pub fn reward(deps: DepsMut, amount: Uint128) -> Result<Response, ContractError> {
    let mut state = load_state(deps.storage)?;
    state.virtual_reward_balance += amount;
    save_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "reward")
        .add_attribute("amount", amount))
}

fn get_time(block: &BlockInfo) -> u64 {
    block.time.seconds()
}

pub fn get_staking_total_balance(
    deps: Deps,
    stake_operator: Option<Addr>,
    state: &State,
) -> Result<Uint128, ContractError> {
    Ok(if let Some(stake_operator) = stake_operator {
        let s: StakeOperatorStateResponse = deps
            .querier
            .query_wasm_smart(stake_operator, &StakeOperatorQueryMsg::State {})?;
        s.total_share
    } else {
        state.staking_total_balance
    })
}

pub fn get_staker_balance(
    deps: Deps,
    stake_operator: Option<Addr>,
    staker: &Staker,
    addr: &Addr,
) -> Result<Uint128, ContractError> {
    Ok(if let Some(stake_operator) = stake_operator {
        let s: StakeOperatorStakerResponse = deps.querier.query_wasm_smart(
            stake_operator,
            &StakeOperatorQueryMsg::Staker {
                address: addr.to_string(),
            },
        )?;
        s.balance
    } else {
        staker.balance
    })
}

pub fn prism_xprism_swap(
    prism_token: &Addr,
    prism_gov: &Addr,
    amount: Uint128,
    recipient: &Addr,
) -> StdResult<SubMsg> {
    send(
        prism_token,
        prism_gov,
        amount,
        &prism_protocol::gov::Cw20HookMsg::MintXprism {
            receiver: Some(recipient.to_string()),
        },
    )
}

pub fn prism_xprism_swap_and_reply(
    prism_token: &Addr,
    prism_gov: &Addr,
    amount: Uint128,
) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        send_wasm_msg(
            prism_token,
            prism_gov,
            amount,
            &prism_protocol::gov::Cw20HookMsg::MintXprism { receiver: None },
        )?,
        ReplyId::XPrismTokensMinted.into(),
    ))
}
