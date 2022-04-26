use crate::{
    commands,
    msg::Cw20HookMsg,
    state::{
        load_config, load_gov_update, load_withdraw_action, remove_gov_update,
        remove_withdraw_action, store_config, store_gov_update, store_withdraw_action, Config,
        GovernanceUpdateState, WithdrawAction,
    },
    SubmsgIds,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    from_binary, to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use nexus_prism_protocol::common::{get_time, query_token_balance, query_token_supply};

pub fn update_config(
    deps: DepsMut,
    mut config: Config,
    compounding_token: Option<String>,
    auto_compounding_token: Option<String>,
    reward_token: Option<String>,
    reward_compound_pair: Option<String>,
    rewards_contract: Option<String>,
) -> StdResult<Response> {
    if let Some(compounding_token) = compounding_token {
        config.compounding_token = deps.api.addr_validate(&compounding_token)?;
    }

    if let Some(auto_compounding_token) = auto_compounding_token {
        config.auto_compounding_token = deps.api.addr_validate(&auto_compounding_token)?;
    }

    if let Some(reward_token) = reward_token {
        config.reward_token = deps.api.addr_validate(&reward_token)?;
    }

    if let Some(reward_compound_pair) = reward_compound_pair {
        config.reward_compound_pair = deps.api.addr_validate(&reward_compound_pair)?;
    }

    if let Some(rewards_contract) = rewards_contract {
        config.rewards_contract = deps.api.addr_validate(&rewards_contract)?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::default())
}

pub fn update_governance_addr(
    deps: DepsMut,
    env: Env,
    gov_addr: String,
    seconds_to_wait_for_accept_gov_tx: u64,
) -> StdResult<Response> {
    let current_time = get_time(&env.block);
    let gov_update = GovernanceUpdateState {
        new_governance_contract_addr: deps.api.addr_validate(&gov_addr)?,
        wait_approve_until: current_time + seconds_to_wait_for_accept_gov_tx,
    };
    store_gov_update(deps.storage, &gov_update)?;
    Ok(Response::default())
}

pub fn accept_governance(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let gov_update = load_gov_update(deps.storage)?;
    let current_time = get_time(&env.block);

    if gov_update.wait_approve_until < current_time {
        return Err(StdError::generic_err(
            "too late to accept governance owning",
        ));
    }

    if info.sender != gov_update.new_governance_contract_addr {
        return Err(StdError::generic_err("unauthorized"));
    }

    let new_gov_add_str = gov_update.new_governance_contract_addr.to_string();

    let mut config = load_config(deps.storage)?;
    config.governance_contract = gov_update.new_governance_contract_addr;
    store_config(deps.storage, &config)?;
    remove_gov_update(deps.storage);

    Ok(Response::default().add_attributes(vec![
        ("action", "change_governance_contract"),
        ("new_address", &new_gov_add_str),
    ]))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::Deposit {} => commands::receive_cw20_deposit(deps, env, info, cw20_msg),
        Cw20HookMsg::Withdraw {} => commands::receive_cw20_withdraw(deps, env, info, cw20_msg),
    }
}

pub fn receive_cw20_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let config: Config = load_config(deps.storage)?;
    if info.sender != config.compounding_token {
        return Err(StdError::generic_err("unauthorized"));
    }

    deposit_compounding_token(
        deps,
        env,
        config,
        Addr::unchecked(cw20_msg.sender),
        cw20_msg.amount.into(),
    )
}

pub fn deposit_compounding_token(
    deps: DepsMut,
    env: Env,
    config: Config,
    farmer: Addr,
    amount: Uint256,
) -> StdResult<Response> {
    let auto_compounding_token_supply: Uint256 =
        query_token_supply(deps.as_ref(), &config.auto_compounding_token)?.into();

    let compounding_token_balance: Uint256 = query_token_balance(
        deps.as_ref(),
        &config.compounding_token,
        &env.contract.address,
    )
    .into();

    let is_first_depositor = auto_compounding_token_supply.is_zero();

    let auto_compounding_token_to_mint = if is_first_depositor {
        amount
    } else {
        auto_compounding_token_supply * amount
            / Decimal256::from_uint256(compounding_token_balance - amount)
    };

    Ok(Response::new()
        .add_message(WasmMsg::Execute {
            contract_addr: config.auto_compounding_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: farmer.to_string(),
                amount: auto_compounding_token_to_mint.into(),
            })?,
            funds: vec![],
        })
        .add_attribute("action", "deposit_compounding_token")
        .add_attribute("farmer", farmer)
        .add_attribute("amount", amount))
}

pub fn receive_cw20_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let config: Config = load_config(deps.storage)?;
    if info.sender != config.auto_compounding_token {
        return Err(StdError::generic_err("unauthorized"));
    }

    withdraw_compounding_token(
        deps,
        env,
        config,
        Addr::unchecked(cw20_msg.sender),
        cw20_msg.amount,
    )
}

pub fn withdraw_compounding_token(
    deps: DepsMut,
    _env: Env,
    config: Config,
    farmer: Addr,
    amount: Uint128,
) -> StdResult<Response> {
    store_withdraw_action(
        deps.storage,
        WithdrawAction {
            farmer,
            auto_compounding_token_amount: amount,
        },
    )?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_always(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.rewards_contract.to_string(),
                msg: to_binary(&todo!())?,
                funds: vec![],
            }),
            SubmsgIds::RewardsClaimed.id(),
        ))
        .add_attributes(vec![("action", "claim_rewards")]))
}

pub fn compound(deps: DepsMut, _env: Env, _info: MessageInfo) -> StdResult<Response> {
    let config: Config = load_config(deps.storage)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.rewards_contract.to_string(),
                msg: to_binary(&todo!())?,
                funds: vec![],
            }),
            SubmsgIds::RewardsClaimed.id(),
        ))
        .add_attributes(vec![("action", "claim_rewards")]))
}

pub fn execute_withdraw(deps: DepsMut, env: Env) -> StdResult<Response> {
    let config = load_config(deps.storage)?;
    if let Some(withdraw_action) = load_withdraw_action(deps.storage)? {
        remove_withdraw_action(deps.storage)?;

        let compounding_token_balance: Uint256 = query_token_balance(
            deps.as_ref(),
            &config.compounding_token,
            &env.contract.address,
        )
        .into();

        let auto_compounding_token_supply: Uint256 =
            query_token_supply(deps.as_ref(), &config.auto_compounding_token)?.into();

        let compounding_token_to_withdraw: Uint256 = compounding_token_balance
            * Uint256::from(withdraw_action.auto_compounding_token_amount)
            / Decimal256::from_uint256(auto_compounding_token_supply);

        Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.compounding_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: withdraw_action.farmer.to_string(),
                    amount: compounding_token_to_withdraw.into(),
                })?,
                funds: vec![],
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.auto_compounding_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: withdraw_action.auto_compounding_token_amount,
                })?,
                funds: vec![],
            }))
            .add_attribute("action", "withdraw")
            .add_attribute(
                "compounding_token_amount_withdrawn",
                compounding_token_to_withdraw,
            )
            .add_attribute(
                "auto_compounding_token_amount_burned",
                withdraw_action.auto_compounding_token_amount,
            ))
    } else {
        Ok(Response::new())
    }
}
