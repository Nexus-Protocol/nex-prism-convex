use crate::queries::{
    query_auto_compounding_token_value, query_compounding_token_value, query_config,
};
use crate::replies_id::ReplyId;
use crate::reply_response::MsgInstantiateContractResponse;
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg,
};
use cw0::nonpayable;
use cw2::{get_contract_version, set_contract_version};
use nexus_prism_protocol::common::{instantiate_token, query_token_balance, send_wasm_msg};
use protobuf::Message;

use crate::commands::{
    accept_governance, compound, receive_cw20_deposit, receive_cw20_withdraw, update_config,
    withdraw,
};
use crate::state::Config;
use crate::{
    commands,
    state::{load_config, remove_withdraw_action, store_config},
};
use cw20::{Cw20ReceiveMsg, TokenInfoResponse};
use nexus_prism_protocol::autocompounder::{
    Cw20HookMsg, ExecuteMsg, GovernanceMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use std::convert::TryFrom;

const CONTRACT_NAME: &str = "nexus.protocol:nex-prism-autocompounder";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    nonpayable(&info).map_err(|_| StdError::generic_err("no payment needed"))?;

    let config = Config {
        compounding_token: deps.api.addr_validate(&msg.compounding_token)?,
        auto_compounding_token: Addr::unchecked(""),
        reward_token: deps.api.addr_validate(&msg.reward_token)?,
        reward_compound_pair: deps.api.addr_validate(&msg.reward_compound_pair)?,
        governance: deps.api.addr_validate(&msg.governance)?,
        staking_contract: deps.api.addr_validate(&msg.staking_contract)?,
    };
    store_config(deps.storage, &config)?;
    remove_withdraw_action(deps.storage)?;

    let compounder_token_info: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(config.compounding_token, &cw20::Cw20QueryMsg::TokenInfo {})?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            instantiate_token(
                &config.governance,
                msg.cw20_token_code_id,
                format!(
                    "{} autocompounder share representation",
                    compounder_token_info.symbol,
                ),
                format!("c{}", compounder_token_info.symbol),
                &env.contract.address,
            )?,
            ReplyId::AutoCompoundingTokenCreated.into(),
        ))
        .add_attribute("action", "instantiate"))
}

fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::Deposit {} => receive_cw20_deposit(deps, env, info, cw20_msg),
        Cw20HookMsg::Withdraw {} => receive_cw20_withdraw(deps, env, info, cw20_msg),
    }
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    nonpayable(&info).map_err(|_| StdError::generic_err("no payment needed"))?;

    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Compound {} => compound(deps, env, info),

        ExecuteMsg::AcceptGovernance {} => accept_governance(deps, env, info),

        ExecuteMsg::Governance { governance_msg } => {
            let config: Config = load_config(deps.storage)?;
            if info.sender != config.governance {
                return Err(StdError::generic_err("unauthorized"));
            }

            match governance_msg {
                GovernanceMsg::UpdateConfig {
                    reward_compound_pair,
                    staking_contract,
                } => update_config(deps, config, reward_compound_pair, staking_contract),

                GovernanceMsg::UpdateGovernanceContract {
                    gov_addr,
                    seconds_to_wait_for_accept_gov_tx,
                } => commands::update_governance(
                    deps,
                    env,
                    gov_addr,
                    seconds_to_wait_for_accept_gov_tx,
                ),
            }
        }
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    let reply_id =
        ReplyId::try_from(msg.id).map_err(|_| StdError::generic_err("unknown reply id"))?;

    let mut config = load_config(deps.storage)?;

    match reply_id {
        ReplyId::AutoCompoundingTokenCreated => {
            let data = msg.result.unwrap().data.unwrap();
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(data.as_slice())
                .map_err(|_| {
                    StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
                })?;

            config.auto_compounding_token = Addr::unchecked(res.get_contract_address());
            store_config(deps.storage, &config)?;

            Ok(Response::new()
                .add_attribute("action", "auto_compounding_token_instantiated")
                .add_attribute("auto_compounding_token", config.auto_compounding_token))
        }

        ReplyId::RewardsClaimed => {
            let reward_token_balance =
                query_token_balance(deps.as_ref(), &config.reward_token, &env.contract.address);

            if reward_token_balance.is_zero() {
                return withdraw(deps, env);
            }

            Ok(Response::new().add_submessage(SubMsg::reply_on_success(
                send_wasm_msg(
                    &config.reward_token,
                    &config.reward_compound_pair,
                    reward_token_balance,
                    &astroport::pair::Cw20HookMsg::Swap {
                        belief_price: None,
                        max_spread: None,
                        to: None,
                    },
                )?,
                ReplyId::RewardsSold.into(),
            )))
        }

        ReplyId::RewardsSold => withdraw(deps, env),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AutoCompoundingTokenValue { amount } => {
            to_binary(&query_auto_compounding_token_value(deps, env, amount)?)
        }
        QueryMsg::CompoundingTokenValue { amount } => {
            to_binary(&query_compounding_token_value(deps, env, amount)?)
        }
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
