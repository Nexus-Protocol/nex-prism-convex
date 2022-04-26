use cosmwasm_bignumber::Decimal256;
use cosmwasm_storage::to_length_prefixed;
use cw20::Cw20ExecuteMsg;
use cw20_base::state::TokenInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, Binary, BlockInfo, Decimal, Deps, Order, QueryRequest, StdError, StdResult,
    SubMsg, Uint128, WasmMsg, WasmQuery,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderBy {
    Asc,
    Desc,
}

impl From<OrderBy> for Order {
    fn from(val: OrderBy) -> Self {
        if val == OrderBy::Asc {
            Order::Ascending
        } else {
            Order::Descending
        }
    }
}

pub fn get_price(deps: Deps, pair: &Addr, token1: &Addr, token2: &Addr) -> StdResult<Decimal> {
    let balance1 = query_token_balance(deps, pair, token1);
    let balance2 = query_token_balance(deps, pair, token2);
    if balance1.is_zero() || balance2.is_zero() {
        return Err(StdError::generic_err(format!("no tokens in pair {}", pair)));
    }
    Ok(Decimal::from_ratio(balance2, balance1))
}

pub fn query_token_balance(deps: Deps, contract_addr: &Addr, account_addr: &Addr) -> Uint128 {
    if let Ok(balance) = query_token_balance_legacy(deps, contract_addr, account_addr) {
        return balance;
    }

    if let Ok(balance) = query_token_balance_new(deps, contract_addr, account_addr) {
        return balance;
    }

    Uint128::zero()
}

fn query_token_balance_new(
    deps: Deps,
    contract_addr: &Addr,
    account_addr: &Addr,
) -> StdResult<Uint128> {
    // load balance form the cw20 token contract version 0.6+
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: contract_addr.to_string(),
        key: Binary::from(concat(
            &to_length_prefixed(b"balance"),
            account_addr.as_bytes(),
        )),
    }))
}

fn query_token_balance_legacy(
    deps: Deps,
    contract_addr: &Addr,
    account_addr: &Addr,
) -> StdResult<Uint128> {
    // load balance form the cw20 token contract version 0.2.x
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: contract_addr.to_string(),
        key: Binary::from(concat(
            &to_length_prefixed(b"balance"),
            (deps.api.addr_canonicalize(account_addr.as_str())?).as_slice(),
        )),
    }))
}

pub fn query_token_supply(deps: Deps, contract_addr: &Addr) -> StdResult<Uint128> {
    if let Ok(supply) = query_token_supply_legacy(deps, contract_addr) {
        return Ok(supply);
    }

    query_token_supply_new(deps, contract_addr)
}

fn query_token_supply_new(deps: Deps, contract_addr: &Addr) -> StdResult<Uint128> {
    let token_info: TokenInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: contract_addr.to_string(),
        key: Binary::from(b"token_info"),
    }))?;

    Ok(token_info.total_supply)
}

fn query_token_supply_legacy(deps: Deps, contract_addr: &Addr) -> StdResult<Uint128> {
    let token_info: TokenInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: contract_addr.to_string(),
        key: Binary::from(to_length_prefixed(b"token_info")),
    }))?;

    Ok(token_info.total_supply)
}

pub fn mint(token: String, recipient: String, amount: Uint128) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: token,
        msg: to_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
        funds: vec![],
    }))
}

pub fn transfer(token: String, recipient: String, amount: Uint128) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: token,
        msg: to_binary(&Cw20ExecuteMsg::Transfer { recipient, amount })?,
        funds: vec![],
    }))
}

#[inline]
fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut k = namespace.to_vec();
    k.extend_from_slice(key);
    k
}

pub fn mul(a: Decimal, b: Decimal) -> Decimal {
    let a: Decimal256 = a.into();
    let b: Decimal256 = b.into();
    (a * b).into()
}

pub fn div(a: Decimal, b: Decimal) -> Decimal {
    let a: Decimal256 = a.into();
    let b: Decimal256 = b.into();
    (a / b).into()
}

pub fn get_time(block: &BlockInfo) -> u64 {
    block.time.seconds()
}
