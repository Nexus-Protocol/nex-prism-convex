use crate::reply_response::MsgInstantiateContractResponse;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use nexus_prism_protocol::common::{query_token_balance, query_token_supply};
use protobuf::Message;

use crate::commands::get_compounding_token_balance;
use crate::msg::{
    AstroportCw20HookMsg, AutoCompoundingTokenValueResponse, CompoundingTokenValueResponse,
    ConfigResponse, ExecuteMsg, GovernanceMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{config_set_auto_compounding_token, Config};
use crate::{
    commands,
    state::{load_config, remove_withdraw_action, store_config},
    SubmsgIds,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cw20::{Cw20ExecuteMsg, MinterResponse, TokenInfoResponse};
use std::convert::TryFrom;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        compounding_token: deps.api.addr_validate(&msg.compounding_token)?,
        auto_compounding_token: Addr::unchecked(""),
        reward_token: deps.api.addr_validate(&msg.reward_token)?,
        reward_compound_pair: deps.api.addr_validate(&msg.reward_compound_pair)?,
        governance_contract: deps.api.addr_validate(&msg.governance_contract_addr)?,
        rewards_contract: deps.api.addr_validate(&msg.rewards_contract)?,
        staking_contract: deps.api.addr_validate(&msg.staking_contract)?,
    };
    store_config(deps.storage, &config)?;
    remove_withdraw_action(deps.storage)?;

    let compounder_token_info: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(config.compounding_token, &cw20::Cw20QueryMsg::TokenInfo {})?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(config.governance_contract.to_string()),
                code_id: msg.cw20_token_code_id,
                msg: to_binary(&cw20_base::msg::InstantiateMsg {
                    name: format!(
                        "{} autocompounder share representation",
                        compounder_token_info.symbol,
                    ),
                    symbol: format!("c{}", compounder_token_info.symbol),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                    marketing: None,
                })?,
                funds: vec![],
                label: "".to_string(),
            }),
            SubmsgIds::InitAutoCompoundingToken.id(),
        ))
        .add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    let submessage_enum = SubmsgIds::try_from(msg.id)?;
    match submessage_enum {
        SubmsgIds::InitAutoCompoundingToken => {
            let data = msg.result.unwrap().data.unwrap();
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(data.as_slice())
                .map_err(|_| {
                    StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
                })?;

            let auto_compounding_token = res.get_contract_address();
            config_set_auto_compounding_token(
                deps.storage,
                Addr::unchecked(auto_compounding_token),
            )?;

            Ok(Response::new().add_attributes(vec![
                ("action", "auto_compounding_token_initialized"),
                ("auto_compounding_token_addr", auto_compounding_token),
            ]))
        }

        SubmsgIds::RewardsClaimed => {
            let config = load_config(deps.storage)?;
            let reward_token_balance =
                query_token_balance(deps.as_ref(), &config.reward_token, &env.contract.address);

            if reward_token_balance.is_zero() {
                return commands::execute_withdraw(deps, env);
            }

            Ok(Response::new().add_submessage(SubMsg::reply_on_success(
                WasmMsg::Execute {
                    contract_addr: config.reward_token.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        amount: reward_token_balance,
                        contract: config.reward_compound_pair.to_string(),
                        msg: to_binary(&AstroportCw20HookMsg::Swap {
                            belief_price: None,
                            max_spread: None,
                            to: None,
                        })?,
                    })?,
                    funds: vec![],
                },
                SubmsgIds::RewardsSold.id(),
            )))
        }

        SubmsgIds::RewardsSold => commands::execute_withdraw(deps, env),
    }
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => commands::receive_cw20(deps, env, info, msg),
        ExecuteMsg::Compound {} => commands::compound(deps, env, info),

        ExecuteMsg::AcceptGovernance {} => commands::accept_governance(deps, env, info),

        ExecuteMsg::Governance { governance_msg } => {
            let config: Config = load_config(deps.storage)?;
            if info.sender != config.governance_contract {
                return Err(StdError::generic_err("unauthorized"));
            }

            match governance_msg {
                GovernanceMsg::UpdateConfig {
                    compounding_token,
                    auto_compounding_token,
                    reward_token,
                    reward_compound_pair,
                    rewards_contract,
                    staking_contract,
                } => commands::update_config(
                    deps,
                    config,
                    compounding_token,
                    auto_compounding_token,
                    reward_token,
                    reward_compound_pair,
                    rewards_contract,
                    staking_contract,
                ),

                GovernanceMsg::UpdateGovernanceContract {
                    gov_addr,
                    seconds_to_wait_for_accept_gov_tx,
                } => commands::update_governance_addr(
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

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = load_config(deps.storage)?;
    Ok(ConfigResponse {
        compounding_token: config.compounding_token.to_string(),
        auto_compounding_token: config.auto_compounding_token.to_string(),
        reward_token: config.reward_token.to_string(),
        reward_compound_pair: config.reward_compound_pair.to_string(),
        governance_contract_addr: config.governance_contract.to_string(),
        rewards_contract: config.rewards_contract.to_string(),
        staking_contract: config.staking_contract.to_string(),
    })
}

pub fn query_auto_compounding_token_value(
    deps: Deps,
    env: Env,
    amount: Uint128,
) -> StdResult<AutoCompoundingTokenValueResponse> {
    let config: Config = load_config(deps.storage)?;

    let compounding_token_balance: Uint256 =
        get_compounding_token_balance(deps, env, &config.staking_contract)?.into();

    let auto_compounding_token_supply: Uint256 =
        query_token_supply(deps, &config.auto_compounding_token)?.into();

    let compounding_token_amount: Uint256 = compounding_token_balance * Uint256::from(amount)
        / Decimal256::from_uint256(auto_compounding_token_supply);

    Ok(AutoCompoundingTokenValueResponse {
        compounding_token_amount: compounding_token_amount.into(),
    })
}

pub fn query_compounding_token_value(
    deps: Deps,
    env: Env,
    amount: Uint128,
) -> StdResult<CompoundingTokenValueResponse> {
    let config: Config = load_config(deps.storage)?;

    let compounding_token_balance: Uint256 =
        get_compounding_token_balance(deps, env, &config.staking_contract)?.into();

    let auto_compounding_token_supply: Uint256 =
        query_token_supply(deps, &config.auto_compounding_token)?.into();

    let auto_compounding_token_amount: Uint256 = auto_compounding_token_supply
        * Uint256::from(amount)
        / Decimal256::from_uint256(compounding_token_balance);

    Ok(CompoundingTokenValueResponse {
        auto_compounding_token_amount: auto_compounding_token_amount.into(),
    })
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
