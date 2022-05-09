use crate::{
    replies_id::ReplyId,
    state::{
        load_config, load_gov_update, load_withdraw_action, remove_gov_update,
        remove_withdraw_action, store_config, store_gov_update, store_withdraw_action, Config,
        GovernanceUpdateState, WithdrawAction,
    },
};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use nexus_prism_protocol::{
    cfg_addr,
    common::{burn, get_time, mint, query_token_balance, query_token_supply, send, transfer},
    staking::StakerResponse,
};

pub fn update_config(
    deps: DepsMut,
    mut config: Config,
    reward_compound_pair: Option<String>,
    staking_contract: Option<String>,
) -> StdResult<Response> {
    cfg_addr!(deps, config, reward_compound_pair, staking_contract);
    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn update_governance(
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
    Ok(Response::new())
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
    config.governance = gov_update.new_governance_contract_addr;
    store_config(deps.storage, &config)?;
    remove_gov_update(deps.storage);

    Ok(Response::default().add_attributes(vec![
        ("action", "change_governance_contract"),
        ("new_address", &new_gov_add_str),
    ]))
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

    let compounding_token_balance: Uint256 =
        get_compounding_token_balance(deps.as_ref(), env, &config.staking_contract)?.into();

    let is_first_depositor = auto_compounding_token_supply.is_zero();

    let auto_compounding_token_to_mint = if is_first_depositor {
        amount
    } else {
        auto_compounding_token_supply * amount / Decimal256::from_uint256(compounding_token_balance)
    };

    Ok(Response::new()
        .add_submessage(mint(
            &config.auto_compounding_token,
            &farmer,
            auto_compounding_token_to_mint.into(),
        )?)
        .add_submessage(send(
            &config.compounding_token,
            &config.staking_contract,
            amount.into(),
            &nexus_prism_protocol::staking::Cw20HookMsg::Bond {},
        )?)
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
        .add_submessage(claim_rewards(&config.staking_contract)?)
        .add_attributes(vec![("action", "claim_rewards")]))
}

pub fn compound(deps: DepsMut, _env: Env, _info: MessageInfo) -> StdResult<Response> {
    let config: Config = load_config(deps.storage)?;

    Ok(Response::new()
        .add_submessage(claim_rewards(&config.staking_contract)?)
        .add_attributes(vec![("action", "claim_rewards")]))
}

fn claim_rewards(staking_contract: &Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_contract.to_string(),
            msg: to_binary(&nexus_prism_protocol::staking::ExecuteMsg::Anyone {
                anyone_msg: nexus_prism_protocol::staking::AnyoneMsg::ClaimRewards {
                    recipient: None,
                },
            })?,
            funds: vec![],
        }),
        ReplyId::RewardsClaimed.into(),
    ))
}

pub fn withdraw(deps: DepsMut, env: Env) -> StdResult<Response> {
    let config = load_config(deps.storage)?;

    let compounding_token_balance = query_token_balance(
        deps.as_ref(),
        &config.compounding_token,
        &env.contract.address,
    );

    let resp = if compounding_token_balance.is_zero() {
        Response::new()
    } else {
        Response::new().add_submessage(send(
            &config.compounding_token,
            &config.staking_contract,
            compounding_token_balance,
            &nexus_prism_protocol::staking::Cw20HookMsg::Bond {},
        )?)
    };

    if let Some(withdraw_action) = load_withdraw_action(deps.storage)? {
        remove_withdraw_action(deps.storage)?;

        let compounding_token_balance: Uint256 =
            get_compounding_token_balance(deps.as_ref(), env, &config.staking_contract)?.into();

        let auto_compounding_token_supply: Uint256 =
            query_token_supply(deps.as_ref(), &config.auto_compounding_token)?.into();

        let compounding_token_to_withdraw: Uint256 = compounding_token_balance
            * Uint256::from(withdraw_action.auto_compounding_token_amount)
            / Decimal256::from_uint256(auto_compounding_token_supply);

        Ok(resp
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.staking_contract.to_string(),
                msg: to_binary(&nexus_prism_protocol::staking::ExecuteMsg::Anyone {
                    anyone_msg: nexus_prism_protocol::staking::AnyoneMsg::Unbond {
                        amount: compounding_token_to_withdraw.into(),
                    },
                })?,
                funds: vec![],
            }))
            .add_submessage(transfer(
                &config.compounding_token,
                &withdraw_action.farmer,
                compounding_token_to_withdraw.into(),
            )?)
            .add_submessage(burn(
                &config.auto_compounding_token,
                withdraw_action.auto_compounding_token_amount,
            )?)
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
        Ok(resp)
    }
}
pub fn get_compounding_token_balance(deps: Deps, env: Env, addr: &Addr) -> StdResult<Uint128> {
    let staker: StakerResponse = deps.querier.query_wasm_smart(
        addr,
        &nexus_prism_protocol::staking::QueryMsg::Staker {
            address: env.contract.address.to_string(),
        },
    )?;
    Ok(staker.balance)
}
