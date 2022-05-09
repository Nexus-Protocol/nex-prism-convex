use std::convert::TryFrom;

use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, Reply, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use nexus_prism_protocol::common::{query_token_balance, transfer};
use protobuf::Message;

use crate::commands::{
    calc_stakers_rewards, distribute_virtual_rewards, get_locked_vested_prism_amount,
    prism_vesting_schedules, update_staking_global_index,
};
use crate::state::{load_locked_vested_prism_amount, save_prism_vesting_schedules, State};
use crate::{
    error::ContractError,
    replies_id::ReplyId,
    reply_response::MsgInstantiateContractResponse,
    state::{load_config, load_state, save_config, Config, InstantiationConfig, INST_CONFIG},
};

fn get_addr(msg: Reply) -> StdResult<Addr> {
    let data = msg.result.unwrap().data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;
    Ok(Addr::unchecked(res.get_contract_address()))
}

fn get_pair_addr(msg: Reply) -> StdResult<Addr> {
    let events = msg
        .result
        .into_result()
        .map_err(|err| StdError::generic_err(format!("Error creating pair: {}", err)))?
        .events;

    events
        .into_iter()
        .flat_map(|event| event.attributes)
        .find(|attr| attr.key == "pair_contract_addr")
        .map(|attr| Addr::unchecked(attr.value))
        .ok_or_else(|| StdError::generic_err("Failed to create pair"))
}

#[allow(clippy::too_many_arguments)]
fn instantiate_staking(
    env: &Env,
    inst_config: &InstantiationConfig,
    config: &Config,
    stake_operator: Option<&Addr>,
    staking_token: &Addr,
    prism_governance: Option<&Addr>,
    is_nexprism_xprism: bool,
    reply_id: ReplyId,
) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(inst_config.admin.to_string()),
            code_id: inst_config.staking_code_id,
            msg: to_binary(&nexus_prism_protocol::staking::InstantiateMsg {
                stake_operator: stake_operator.map(|x| x.to_string()),
                staking_token: staking_token.to_string(),
                reward_operator: env.contract.address.to_string(),
                reward_token: config.prism_token.to_string(),
                prism_governance: prism_governance.map(|x| x.to_string()),
                governance: config.governance.to_string(),
                xprism_token: if is_nexprism_xprism {
                    Some(config.xprism_token.to_string())
                } else {
                    None
                },
                nexprism_xprism_pair: if is_nexprism_xprism {
                    Some(inst_config.nexprism_xprism_pair.to_string())
                } else {
                    None
                },
            })?,
            funds: vec![],
            label: "".to_owned(),
        }),
        reply_id.into(),
    ))
}

fn instantiate_nyluna_staking(
    env: &Env,
    inst_config: &InstantiationConfig,
    config: &Config,
) -> StdResult<SubMsg> {
    instantiate_staking(
        env,
        inst_config,
        config,
        None,
        &config.nyluna_token,
        None,
        false,
        ReplyId::NYLunaStakingCreated,
    )
}

fn instantiate_nexprism_staking(
    env: &Env,
    inst_config: &InstantiationConfig,
    config: &Config,
) -> StdResult<SubMsg> {
    instantiate_staking(
        env,
        inst_config,
        config,
        None,
        &config.nexprism_token,
        Some(&inst_config.prism_governance),
        false,
        ReplyId::NexPrismStakingCreated,
    )
}

fn instantiate_psi_staking(
    env: &Env,
    inst_config: &InstantiationConfig,
    config: &Config,
) -> StdResult<SubMsg> {
    instantiate_staking(
        env,
        inst_config,
        config,
        Some(&config.governance),
        &inst_config.psi_token,
        Some(&inst_config.prism_governance),
        true,
        ReplyId::PsiStakingCreated,
    )
}

fn instantiate_autocompounder(
    inst_config: &InstantiationConfig,
    config: &Config,
    compounding_token: &Addr,
    reward_token: &Addr,
    reward_compound_pair: &Addr,
    staking_contract: &Addr,
    reply_id: ReplyId,
) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(inst_config.admin.to_string()),
            code_id: inst_config.autocompounder_code_id,
            msg: to_binary(&nexus_prism_protocol::autocompounder::InstantiateMsg {
                compounding_token: compounding_token.to_string(),
                reward_token: reward_token.to_string(),
                reward_compound_pair: reward_compound_pair.to_string(),
                governance: config.governance.to_string(),
                staking_contract: staking_contract.to_string(),
                cw20_token_code_id: inst_config.cw20_token_code_id,
            })?,
            funds: vec![],
            label: "".to_owned(),
        }),
        reply_id.into(),
    ))
}

fn instantiate_nexprism_autocompounder(
    inst_config: &InstantiationConfig,
    config: &Config,
) -> StdResult<SubMsg> {
    instantiate_autocompounder(
        inst_config,
        config,
        &config.nexprism_token,
        &config.xprism_token,
        &inst_config.nexprism_xprism_pair,
        &config.nexprism_staking,
        ReplyId::NexPrismAutocompounderCreated,
    )
}

fn instantiate_nyluna_autocompounder(
    inst_config: &InstantiationConfig,
    config: &Config,
) -> StdResult<SubMsg> {
    instantiate_autocompounder(
        inst_config,
        config,
        &config.nyluna_token,
        &config.prism_token,
        &config.prism_yluna_pair,
        &config.nyluna_staking,
        ReplyId::NYLunaAutocompounderCreated,
    )
}

fn instantiate_nexprism_xprism_pair(
    inst_config: &InstantiationConfig,
    config: &Config,
) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: inst_config.astroport_factory.to_string(),
            msg: to_binary(&astroport::factory::ExecuteMsg::CreatePair {
                pair_type: astroport::factory::PairType::Stable {},
                asset_infos: [
                    astroport::asset::AssetInfo::Token {
                        contract_addr: config.xprism_token.clone(),
                    },
                    astroport::asset::AssetInfo::Token {
                        contract_addr: config.nexprism_token.clone(),
                    },
                ],
                init_params: Some(to_binary(&astroport::pair::StablePoolParams {
                    amp: inst_config.nexprism_xprism_amp_coef,
                })?),
            })?,
            funds: vec![],
        }),
        ReplyId::NexPrismXPrismPairCreated.into(),
    ))
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut inst_config = INST_CONFIG.load(deps.storage)?;
    let mut config = load_config(deps.storage)?;
    let state = load_state(deps.storage)?;

    let reply_id =
        ReplyId::try_from(msg.id).map_err(|_| ContractError::UnknownReplyId { id: msg.id })?;

    match reply_id {
        ReplyId::NYLunaTokenCreated => {
            config.nyluna_token = get_addr(msg)?;
            save_config(deps.storage, &config)?;

            Ok(Response::new()
                .add_submessage(instantiate_nyluna_staking(&env, &inst_config, &config)?)
                .add_attribute("action", "nyluna_token_instantiated")
                .add_attribute("nyluna_token", config.nyluna_token))
        }

        ReplyId::NexPrismTokenCreated => {
            config.nexprism_token = get_addr(msg)?;
            save_config(deps.storage, &config)?;

            Ok(Response::new()
                .add_submessage(instantiate_nexprism_staking(&env, &inst_config, &config)?)
                .add_submessage(instantiate_nexprism_xprism_pair(&inst_config, &config)?)
                .add_attribute("action", "nexprism_token_instantiated")
                .add_attribute("nexprism_token", config.nexprism_token))
        }

        ReplyId::NexPrismXPrismPairCreated => {
            inst_config.nexprism_xprism_pair = get_pair_addr(msg)?;
            INST_CONFIG.save(deps.storage, &inst_config)?;

            Ok(Response::new()
                .add_submessage(instantiate_psi_staking(&env, &inst_config, &config)?)
                .add_submessage(instantiate_nexprism_autocompounder(&inst_config, &config)?)
                .add_attribute("action", "nexprism_xprism_pair_instantiated")
                .add_attribute("nexprism_xprism_pair", inst_config.nexprism_xprism_pair))
        }

        ReplyId::NexPrismStakingCreated => {
            config.nexprism_staking = get_addr(msg)?;
            save_config(deps.storage, &config)?;

            Ok(Response::new()
                .add_attribute("action", "nexprism_staking_instantiated")
                .add_attribute("nexprism_staking", config.nexprism_staking))
        }

        ReplyId::PsiStakingCreated => {
            config.psi_staking = get_addr(msg)?;
            save_config(deps.storage, &config)?;

            Ok(Response::new()
                .add_attribute("action", "psi_staking_instantiated")
                .add_attribute("psi_staking", config.psi_staking))
        }

        ReplyId::NYLunaStakingCreated => {
            config.nyluna_staking = get_addr(msg)?;
            save_config(deps.storage, &config)?;

            Ok(Response::new()
                .add_submessage(instantiate_nyluna_autocompounder(&inst_config, &config)?)
                .add_attribute("action", "nyluna_staking_instantiated")
                .add_attribute("nyluna_staking", config.nyluna_staking))
        }

        ReplyId::NexPrismAutocompounderCreated => Ok(Response::new()
            .add_attribute("action", "nexprism_autocompounder_instantiated")
            .add_attribute("nexprism_autocompounder", get_addr(msg)?)),

        ReplyId::NYLunaAutocompounderCreated => Ok(Response::new()
            .add_attribute("action", "nyluna_autocompounder_instantiated")
            .add_attribute("nyluna_autocompounder", get_addr(msg)?)),

        ReplyId::XPrismBoostActivated => match msg.result {
            cosmwasm_std::ContractResult::Err(err_msg) => {
                if err_msg.to_lowercase().contains("nothing bonded") {
                    Ok(xprism_boost_activated_logic(deps, &env, &config)?)
                } else {
                    Err(StdError::generic_err(format!(
                        "fail to activate xprism boost: {}",
                        err_msg
                    ))
                    .into())
                }
            }
            cosmwasm_std::ContractResult::Ok(_) => {
                Ok(xprism_boost_activated_logic(deps, &env, &config)?)
            }
        },

        // update_vest in Prism has been called just before it.
        ReplyId::VirtualRewardsClaimed => {
            let mut resp = Response::new().add_attribute("action", "virtual_rewards_claimed");

            let prev_locked_vested_prism_amount = load_locked_vested_prism_amount(deps.storage)?;
            let prism_vesting_schedules =
                prism_vesting_schedules(deps.as_ref(), &env, &config.prism_launch_pool)?;
            let locked_vested_prism_amount =
                get_locked_vested_prism_amount(env.block.time.seconds(), &prism_vesting_schedules);
            let claimed_rewards = locked_vested_prism_amount - prev_locked_vested_prism_amount;
            resp = distribute_virtual_rewards(&config, &state, claimed_rewards, resp)?;

            save_prism_vesting_schedules(deps.storage, prism_vesting_schedules)?;

            Ok(resp)
        }

        // update_vest in Prism has been called just before it.
        ReplyId::RealRewardsClaimed => match msg.result {
            cosmwasm_std::ContractResult::Err(err_msg) => {
                if err_msg.to_lowercase().contains("no claimable rewards") {
                    Ok(real_rewards_claimed_logic(deps, &env, &config, &state)?)
                } else {
                    Err(
                        StdError::generic_err(format!("fail to claim real rewards: {}", err_msg))
                            .into(),
                    )
                }
            }
            cosmwasm_std::ContractResult::Ok(_) => {
                Ok(real_rewards_claimed_logic(deps, &env, &config, &state)?)
            }
        },
    }
}

fn xprism_boost_activated_logic(
    _deps: DepsMut,
    _env: &Env,
    _config: &Config,
) -> StdResult<Response> {
    Ok(Response::new().add_attribute("action", "xprism_boost_activated"))
}

fn real_rewards_claimed_logic(
    deps: DepsMut,
    env: &Env,
    config: &Config,
    state: &State,
) -> StdResult<Response> {
    // We don`t update PRISM_VESTING_STATE here, because real rewards claim
    // happens just after virtual rewards claim only, where we do it.

    let claimed_rewards =
        query_token_balance(deps.as_ref(), &config.prism_token, &env.contract.address);
    let (nexprism_stakers_rewards, nyluna_stakers_rewards, psi_stakers_rewards) =
        calc_stakers_rewards(state, claimed_rewards);

    let mut resp = Response::new();
    if !nexprism_stakers_rewards.is_zero() {
        resp = resp
            .add_submessage(transfer(
                &config.prism_token,
                &config.nexprism_staking,
                nexprism_stakers_rewards,
            )?)
            .add_submessage(update_staking_global_index(&config.nexprism_staking)?);
    }
    if !nyluna_stakers_rewards.is_zero() {
        resp = resp
            .add_submessage(transfer(
                &config.prism_token,
                &config.nyluna_staking,
                nyluna_stakers_rewards,
            )?)
            .add_submessage(update_staking_global_index(&config.nyluna_staking)?);
    }
    if !psi_stakers_rewards.is_zero() {
        resp = resp
            .add_submessage(transfer(
                &config.prism_token,
                &config.psi_staking,
                psi_stakers_rewards,
            )?)
            .add_submessage(update_staking_global_index(&config.psi_staking)?);
    }

    Ok(resp
        .add_attribute("action", "real_rewards_claimed")
        .add_attribute("real_rewards_total", claimed_rewards)
        .add_attribute("nexprism_stakers_real_rewards", nexprism_stakers_rewards)
        .add_attribute("nyluna_stakers_real_rewards", nyluna_stakers_rewards)
        .add_attribute("psi_stakers_real_rewards", psi_stakers_rewards))
}
