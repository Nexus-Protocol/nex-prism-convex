use crate::commands::{
    accept_governance, claim_real_rewards, claim_virtual_rewards, deposit_xprism, deposit_yluna,
    update_config_by_governance, update_config_by_owner, update_governance,
    update_rewards_distribution_by_owner, update_state, withdraw_yluna,
};
use crate::queries::{query_config, query_state, simulate_update_rewards_distribution};
use crate::replies_id::ReplyId;
use cosmwasm_std::{entry_point, Uint128};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, SubMsg,
};
use cw0::nonpayable;
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use nexus_prism_protocol::common::instantiate_token;
use nexus_prism_protocol::vault::{
    Cw20HookMsg, ExecuteMsg, GovernanceMsg, InstantiateMsg, MigrateMsg, OwnerMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::state::{
    load_config, save_config, save_state, Config, InstantiationConfig, State, INST_CONFIG,
};

const CONTRACT_NAME: &str = "nexus.protocol:nex-prism-vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let inst_config = InstantiationConfig {
        admin: info.sender.clone(),
        cw20_token_code_id: msg.cw20_token_code_id,
        staking_code_id: msg.staking_code_id,
        autocompounder_code_id: msg.autocompounder_code_id,
        astroport_factory: deps.api.addr_validate(&msg.astroport_factory)?,
        nexprism_xprism_amp_coef: msg.nexprism_xprism_amp_coef,
        psi_token: deps.api.addr_validate(&msg.psi_token)?,
        prism_governance: deps.api.addr_validate(&msg.prism_governance)?,
        nexprism_xprism_pair: Addr::unchecked(""),
    };
    INST_CONFIG.save(deps.storage, &inst_config)?;

    let config = Config {
        owner: info.sender,
        governance: deps.api.addr_validate(&msg.governance)?,

        xprism_token: deps.api.addr_validate(&msg.xprism_token)?,
        nexprism_token: Addr::unchecked(""),
        yluna_token: deps.api.addr_validate(&msg.yluna_token)?,
        nyluna_token: Addr::unchecked(""),
        prism_token: deps.api.addr_validate(&msg.prism_token)?,

        prism_launch_pool: deps.api.addr_validate(&msg.prism_launch_pool)?,
        prism_xprism_boost: deps.api.addr_validate(&msg.prism_xprism_boost)?,

        nexprism_staking: Addr::unchecked(""),
        psi_staking: Addr::unchecked(""),
        nyluna_staking: Addr::unchecked(""),

        prism_xprism_pair: deps.api.addr_validate(&msg.prism_xprism_pair)?,
        prism_yluna_pair: deps.api.addr_validate(&msg.prism_yluna_pair)?,

        rewards_distribution_update_period_secs: msg.rewards_distribution_update_period_secs,
        rewards_distribution_update_step: msg.rewards_distribution_update_step,

        min_nexprism_stakers_reward_ratio: msg.min_nexprism_stakers_reward_ratio,
        max_nexprism_stakers_reward_ratio: msg.max_nexprism_stakers_reward_ratio,
        min_nyluna_stakers_reward_ratio: msg.min_nyluna_stakers_reward_ratio,
        max_nyluna_stakers_reward_ratio: msg.max_nyluna_stakers_reward_ratio,
    };
    save_config(deps.storage, &config)?;

    let initial_state = State {
        nexprism_stakers_reward_ratio: msg.nexprism_stakers_reward_ratio,
        nyluna_stakers_reward_ratio: msg.nyluna_stakers_reward_ratio,
        psi_stakers_reward_ratio: msg.psi_stakers_reward_ratio,
        last_calculation_time: 0,
        xprism_amount_total: Uint128::zero(),
        yluna_amount_total: Uint128::zero(),
    };
    save_state(deps.storage, &config, &initial_state)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            instantiate_token(
                &config.governance,
                inst_config.cw20_token_code_id,
                "yLUNA representation in Nexus vault",
                "nyLUNA",
                &env.contract.address,
            )?,
            ReplyId::NYLunaTokenCreated.into(),
        ))
        .add_submessage(SubMsg::reply_on_success(
            instantiate_token(
                &config.governance,
                inst_config.cw20_token_code_id,
                "xPRISM representation in Nexus vault",
                "nexPRISM",
                &env.contract.address,
            )?,
            ReplyId::NexPrismTokenCreated.into(),
        ))
        .add_attribute("action", "instantiate"))
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

        ExecuteMsg::ClaimVirtualRewards {} => claim_virtual_rewards(deps, env, info),
        ExecuteMsg::ClaimRealRewards {} => claim_real_rewards(deps, env, info),

        ExecuteMsg::Owner { msg } => {
            let config = load_config(deps.storage)?;
            if info.sender != config.owner {
                return Err(ContractError::Unauthorized {});
            }
            match msg {
                OwnerMsg::UpdateRewardsDistribution {} => {
                    update_rewards_distribution_by_owner(deps, env, config)
                }
                OwnerMsg::UpdateState {
                    nexprism_stakers_reward_ratio,
                    nyluna_stakers_reward_ratio,
                    psi_stakers_reward_ratio,
                    last_calculation_time,
                } => update_state(
                    deps,
                    config,
                    nexprism_stakers_reward_ratio,
                    nyluna_stakers_reward_ratio,
                    psi_stakers_reward_ratio,
                    last_calculation_time,
                ),
                OwnerMsg::UpdateConfig {
                    owner,
                    prism_launch_pool,
                    prism_xprism_boost,
                    prism_xprism_pair,
                    prism_yluna_pair,
                    rewards_distribution_update_period_secs,
                    rewards_distribution_update_step,
                    min_nexprism_stakers_reward_ratio,
                    max_nexprism_stakers_reward_ratio,
                    min_nyluna_stakers_reward_ratio,
                    max_nyluna_stakers_reward_ratio,
                } => update_config_by_owner(
                    deps,
                    config,
                    owner,
                    prism_launch_pool,
                    prism_xprism_boost,
                    prism_xprism_pair,
                    prism_yluna_pair,
                    rewards_distribution_update_period_secs,
                    rewards_distribution_update_step,
                    min_nexprism_stakers_reward_ratio,
                    max_nexprism_stakers_reward_ratio,
                    min_nyluna_stakers_reward_ratio,
                    max_nyluna_stakers_reward_ratio,
                ),
            }
        }

        ExecuteMsg::AcceptGovernance {} => accept_governance(deps, env, info),
        ExecuteMsg::Governance { msg } => {
            let config = load_config(deps.storage)?;
            if info.sender != config.governance {
                return Err(ContractError::Unauthorized {});
            }
            match msg {
                GovernanceMsg::UpdateState {
                    nexprism_stakers_reward_ratio,
                    nyluna_stakers_reward_ratio,
                    psi_stakers_reward_ratio,
                    last_calculation_time,
                } => update_state(
                    deps,
                    config,
                    nexprism_stakers_reward_ratio,
                    nyluna_stakers_reward_ratio,
                    psi_stakers_reward_ratio,
                    last_calculation_time,
                ),
                GovernanceMsg::UpdateConfig {
                    owner,
                    rewards_distribution_update_period_secs,
                    rewards_distribution_update_step,
                    min_nexprism_stakers_reward_ratio,
                    max_nexprism_stakers_reward_ratio,
                    min_nyluna_stakers_reward_ratio,
                    max_nyluna_stakers_reward_ratio,
                } => update_config_by_governance(
                    deps,
                    config,
                    owner,
                    rewards_distribution_update_period_secs,
                    rewards_distribution_update_step,
                    min_nexprism_stakers_reward_ratio,
                    max_nexprism_stakers_reward_ratio,
                    min_nyluna_stakers_reward_ratio,
                    max_nyluna_stakers_reward_ratio,
                ),
                GovernanceMsg::UpdateGovernance {
                    addr,
                    seconds_to_wait_for_accept_gov_tx,
                } => update_governance(deps, env, addr, seconds_to_wait_for_accept_gov_tx),
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
    let config = load_config(deps.storage)?;
    let token = info.sender.clone();

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Deposit {}) if token == config.xprism_token => {
            deposit_xprism(deps, env, info, config, cw20_msg.sender, cw20_msg.amount)
        }
        Ok(Cw20HookMsg::Deposit {}) if token == config.yluna_token => {
            deposit_yluna(deps, env, info, config, cw20_msg.sender, cw20_msg.amount)
        }
        Ok(Cw20HookMsg::Deposit {}) => Err(ContractError::Unauthorized {}),

        Ok(Cw20HookMsg::Withdraw {}) if token == config.nyluna_token => {
            withdraw_yluna(deps, env, info, config, cw20_msg.sender, cw20_msg.amount)
        }
        Ok(Cw20HookMsg::Withdraw {}) => Err(ContractError::Unauthorized {}),

        Err(err) => Err(ContractError::Std(err)),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::SimulateUpdateRewardsDistribution {} => {
            to_binary(&simulate_update_rewards_distribution(deps, env)?)
        }
    }
}

#[entry_point]
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
