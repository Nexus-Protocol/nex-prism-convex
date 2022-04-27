use cosmwasm_std::{
    entry_point, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};

use crate::{
    commands,
    error::ContractError,
    queries,
    state::{load_config, save_config, save_state, RewardState, State},
};
use crate::{state::Config, ContractResult};
use nexus_prism_protocol::staking::{
    AnyoneMsg, ExecuteMsg, GovernanceMsg, InstantiateMsg, MigrateMsg, OwnerMsg, QueryMsg,
    RewarderMsg,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    let config = Config {
        owner: msg
            .owner
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?,
        staking_token: deps.api.addr_validate(&msg.staking_token)?,
        rewarder: deps.api.addr_validate(&msg.rewarder)?,
        reward_token: deps.api.addr_validate(&msg.reward_token)?,
        staker_reward_pair: msg
            .staker_reward_pair
            .into_iter()
            .map(|p| deps.api.addr_validate(&p))
            .collect::<Result<Vec<_>, _>>()?,
        governance: deps.api.addr_validate(&msg.governance)?,
    };

    save_config(deps.storage, &config)?;
    save_state(
        deps.storage,
        &State {
            staking_total_balance: Uint128::zero(),
            virtual_reward_balance: Uint128::zero(),
            virtual_rewards: RewardState {
                global_index: Decimal::zero(),
                prev_balance: Uint128::zero(),
            },
            real_rewards: RewardState {
                global_index: Decimal::zero(),
                prev_balance: Uint128::zero(),
            },
        },
    )?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => commands::receive_cw20(deps, env, info, msg),

        ExecuteMsg::Anyone { anyone_msg } => match anyone_msg {
            AnyoneMsg::Unbond { amount } => commands::unbond(deps, env, info, amount),

            AnyoneMsg::UpdateGlobalIndex {} => commands::update_global_index(deps, env),

            AnyoneMsg::ClaimRewards { recipient } => {
                commands::claim_rewards(deps, env, info, recipient)
            }

            AnyoneMsg::ClaimRewardsForSomeone { address } => {
                commands::claim_rewards_for_someone(deps, env, address)
            }

            AnyoneMsg::AcceptGovernance {} => commands::accept_governance(deps, env, info),
        },

        ExecuteMsg::Owner { msg } => {
            let config = load_config(deps.storage)?;

            if Some(info.sender) != config.owner {
                return Err(ContractError::Unauthorized);
            }

            match msg {
                OwnerMsg::IncreaseBalance { address, amount } => {
                    commands::increase_balance(deps, env, &config, address, amount)
                }

                OwnerMsg::DecreaseBalance { address, amount } => {
                    commands::decrease_balance(deps, env, &config, address, amount)
                }
            }
        }

        ExecuteMsg::Rewarder { msg } => {
            let config = load_config(deps.storage)?;

            if info.sender != config.rewarder {
                return Err(ContractError::Unauthorized);
            }

            match msg {
                RewarderMsg::Reward { amount } => commands::reward(deps, amount),
            }
        }

        ExecuteMsg::Governance { governance_msg } => {
            let config = load_config(deps.storage)?;
            if info.sender != config.governance {
                return Err(ContractError::Unauthorized);
            }

            match governance_msg {
                GovernanceMsg::UpdateConfig {
                    owner,
                    staking_token,
                    rewarder,
                    reward_token,
                    staker_reward_pair,
                } => commands::update_config(
                    deps,
                    config,
                    owner,
                    staking_token,
                    rewarder,
                    reward_token,
                    staker_reward_pair,
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
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Rewards { address } => to_binary(&queries::query_rewards(deps, address)?),
        QueryMsg::Staker { address } => to_binary(&queries::query_staker(deps, env, address)?),
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::new())
}
