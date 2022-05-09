use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Deps, Env, StdResult, Uint128};
use nexus_prism_protocol::{
    autocompounder::{
        AutoCompoundingTokenValueResponse, CompoundingTokenValueResponse, ConfigResponse,
    },
    common::query_token_supply,
};

use crate::{commands::get_compounding_token_balance, state::load_config};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = load_config(deps.storage)?;
    Ok(ConfigResponse {
        compounding_token: config.compounding_token.to_string(),
        auto_compounding_token: config.auto_compounding_token.to_string(),
        reward_token: config.reward_token.to_string(),
        reward_compound_pair: config.reward_compound_pair.to_string(),
        governance: config.governance.to_string(),
        staking_contract: config.staking_contract.to_string(),
    })
}

pub fn query_auto_compounding_token_value(
    deps: Deps,
    env: Env,
    amount: Uint128,
) -> StdResult<AutoCompoundingTokenValueResponse> {
    let config = load_config(deps.storage)?;

    let compounding_token_balance: Uint256 =
        get_compounding_token_balance(deps, env, &config.staking_contract)?.into();

    let auto_compounding_token_supply: Uint256 =
        query_token_supply(deps, &config.auto_compounding_token)?.into();

    if auto_compounding_token_supply.is_zero() {
        return Ok(AutoCompoundingTokenValueResponse {
            compounding_token_amount: Uint256::zero().into(),
        });
    }

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
    let config = load_config(deps.storage)?;

    let compounding_token_balance: Uint256 =
        get_compounding_token_balance(deps, env, &config.staking_contract)?.into();

    let auto_compounding_token_supply: Uint256 =
        query_token_supply(deps, &config.auto_compounding_token)?.into();

    if compounding_token_balance.is_zero() {
        return Ok(CompoundingTokenValueResponse {
            auto_compounding_token_amount: Uint256::zero().into(),
        });
    }

    let auto_compounding_token_amount: Uint256 = auto_compounding_token_supply
        * Uint256::from(amount)
        / Decimal256::from_uint256(compounding_token_balance);

    Ok(CompoundingTokenValueResponse {
        auto_compounding_token_amount: auto_compounding_token_amount.into(),
    })
}
