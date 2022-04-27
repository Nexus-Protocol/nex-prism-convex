use std::cmp::min;

use cosmwasm_std::{
    from_binary, to_binary, Addr, BlockInfo, Decimal, DepsMut, Env, MessageInfo, Response,
    StdError, Uint128, WasmMsg,
};
use cw_asset::Asset;
use nexus_prism_protocol::{
    common::{query_token_balance, transfer},
    staking::Cw20HookMsg,
};

use crate::{
    error::ContractError,
    math::decimal_summation_in_256,
    state::{
        load_config, load_gov_update, load_staker, load_state, remove_gov_update, save_config,
        save_gov_update, save_state, Config, GovernanceUpdateState, RewardState, Staker, State,
    },
    ContractResult,
};
use crate::{
    state::save_staker,
    utils::{calculate_decimal_rewards, get_decimals},
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;

    if config.owner.is_some() || info.sender != config.staking_token {
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

    if config.owner.is_some() {
        return Err(ContractError::Unauthorized);
    }

    Ok(
        decrease_balance(deps, env, &config, info.sender.to_string(), amount)?.add_message(
            WasmMsg::Execute {
                contract_addr: config.staking_token.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount,
                })?,
            },
        ),
    )
}

pub fn update_config(
    deps: DepsMut,
    mut config: Config,
    owner: Option<String>,
    staking_token: Option<String>,
    rewarder: Option<String>,
    reward_token: Option<String>,
    staker_reward_pair: Option<Vec<String>>,
) -> ContractResult<Response> {
    if let Some(owner) = owner {
        config.owner = if owner.is_empty() {
            None
        } else {
            Some(deps.api.addr_validate(&owner)?)
        };
    }

    if let Some(staking_token) = staking_token {
        config.staking_token = deps.api.addr_validate(&staking_token)?;
    }

    if let Some(rewarder) = rewarder {
        config.rewarder = deps.api.addr_validate(&rewarder)?;
    }

    if let Some(reward_token) = reward_token {
        config.reward_token = deps.api.addr_validate(&reward_token)?;
    }

    if let Some(staker_reward_pair) = staker_reward_pair {
        config.staker_reward_pair = staker_reward_pair
            .into_iter()
            .map(|p| deps.api.addr_validate(&p))
            .collect::<Result<Vec<_>, _>>()?;
    }

    save_config(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn update_global_index(deps: DepsMut, env: Env) -> ContractResult<Response> {
    let mut state: State = load_state(deps.storage)?;

    if state.staking_total_balance.is_zero() {
        return Err(ContractError::NoStakers {});
    }

    let config = load_config(deps.storage)?;

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
        return Err(ContractError::NoRewards {});
    }

    save_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "update_global_index")
        .add_attribute("real_claimed_rewards", real_claimed_rewards)
        .add_attribute("virtual_claimed_rewards", virtual_claimed_rewards))
}

pub fn update_governance_addr(
    deps: DepsMut,
    env: Env,
    gov_addr: String,
    seconds_to_wait_for_accept_gov_tx: u64,
) -> ContractResult<Response> {
    let current_time = get_time(&env.block);
    let gov_update = GovernanceUpdateState {
        new_governance_contract_addr: deps.api.addr_validate(&gov_addr)?,
        wait_approve_until: current_time + seconds_to_wait_for_accept_gov_tx,
    };
    save_gov_update(deps.storage, &gov_update)?;
    Ok(Response::new().add_attribute("action", "update_governance_addr"))
}

pub fn accept_governance(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
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
) -> ContractResult<Uint128> {
    let claimed_rewards = reward_token_balance.checked_sub(reward_state.prev_balance)?;

    if staking_token_balance_total.is_zero() {
        return Ok(claimed_rewards);
    }

    reward_state.prev_balance = reward_token_balance;

    reward_state.global_index = decimal_summation_in_256(
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
) -> ContractResult<Response> {
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
) -> ContractResult<Response> {
    let addr = deps.api.addr_validate(&recipient)?;
    claim_rewards_logic(deps, env, &addr, &addr)
}

fn claim_rewards_logic(
    deps: DepsMut,
    env: Env,
    staker_addr: &Addr,
    recipient: &Addr,
) -> ContractResult<Response> {
    let mut staker: Staker = load_staker(deps.storage, staker_addr)?;
    let mut state: State = load_state(deps.storage)?;
    let config: Config = load_config(deps.storage)?;

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

    let all_real_reward_with_decimals: Decimal =
        decimal_summation_in_256(real_reward_with_decimals, staker.real_pending_rewards);
    let real_decimals: Decimal = get_decimals(all_real_reward_with_decimals)?;
    let real_rewards: Uint128 = all_real_reward_with_decimals * Uint128::new(1);

    let all_virtual_reward_with_decimals: Decimal =
        decimal_summation_in_256(virtual_reward_with_decimals, staker.virtual_pending_rewards);
    let virtual_decimals: Decimal = get_decimals(all_virtual_reward_with_decimals)?;
    let virtual_rewards: Uint128 = all_virtual_reward_with_decimals * Uint128::new(1);

    let rewards = min(real_rewards, virtual_rewards);
    if rewards.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    let new_real_balance = state.real_rewards.prev_balance.checked_sub(rewards)?;
    state.real_rewards.prev_balance = new_real_balance;
    let new_virtual_balance = state.virtual_rewards.prev_balance.checked_sub(rewards)?;
    state.virtual_rewards.prev_balance = new_virtual_balance;
    save_state(deps.storage, &state)?;

    staker.real_pending_rewards =
        staker.real_pending_rewards - Decimal::from_ratio(rewards, Uint128::new(1)) + real_decimals;
    staker.real_index = state.real_rewards.global_index;
    staker.virtual_pending_rewards = staker.virtual_pending_rewards
        - Decimal::from_ratio(rewards, Uint128::new(1))
        + virtual_decimals;
    staker.virtual_index = state.virtual_rewards.global_index;
    save_staker(deps.storage, staker_addr, &staker)?;

    let mut resp = Response::new()
        .add_attribute("action", "claim_reward")
        .add_attribute("staker", staker_addr)
        .add_attribute("recipient", recipient)
        .add_attribute("rewards", rewards);

    if !config.staker_reward_pair.is_empty() {
        resp = resp.add_message(Asset::cw20(config.reward_token, rewards).send_msg(
            config.staker_reward_pair[0].clone(),
            to_binary(&astroport::pair::Cw20HookMsg::Swap {
                belief_price: None,
                max_spread: None,
                to: Some(recipient.to_string()),
            })?,
        )?);
    } else {
        resp = resp.add_submessage(transfer(
            config.reward_token.to_string(),
            recipient.to_string(),
            rewards,
        )?);
    }

    Ok(resp)
}

pub fn increase_balance(
    deps: DepsMut,
    env: Env,
    config: &Config,
    address: String,
    amount: Uint128,
) -> ContractResult<Response> {
    let address = deps.api.addr_validate(&address)?;

    let mut state: State = load_state(deps.storage)?;
    let mut staker: Staker = load_staker(deps.storage, &address)?;

    let real_rewards = calculate_decimal_rewards(
        state.real_rewards.global_index,
        staker.real_index,
        staker.balance,
    )?;

    staker.real_index = state.real_rewards.global_index;
    staker.real_pending_rewards =
        decimal_summation_in_256(real_rewards, staker.real_pending_rewards);

    let virtual_rewards = calculate_decimal_rewards(
        state.virtual_rewards.global_index,
        staker.virtual_index,
        staker.balance,
    )?;

    staker.virtual_index = state.virtual_rewards.global_index;
    staker.virtual_pending_rewards =
        decimal_summation_in_256(virtual_rewards, staker.virtual_pending_rewards);

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
) -> ContractResult<Response> {
    let address = deps.api.addr_validate(&address)?;

    let mut state: State = load_state(deps.storage)?;
    let mut staker: Staker = load_staker(deps.storage, &address)?;

    if staker.balance < amount {
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
    staker.real_pending_rewards =
        decimal_summation_in_256(real_rewards, staker.real_pending_rewards);

    let virtual_rewards = calculate_decimal_rewards(
        state.virtual_rewards.global_index,
        staker.virtual_index,
        staker.balance,
    )?;
    staker.virtual_index = state.virtual_rewards.global_index;
    staker.virtual_pending_rewards =
        decimal_summation_in_256(virtual_rewards, staker.virtual_pending_rewards);

    staker.balance = staker.balance.checked_sub(amount)?;
    state.staking_total_balance = state.staking_total_balance.checked_sub(amount)?;

    save_staker(deps.storage, &address, &staker)?;
    save_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "decrease_balance")
        .add_attribute("staker", address)
        .add_attribute("amount", amount))
}

pub fn reward(deps: DepsMut, amount: Uint128) -> ContractResult<Response> {
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
