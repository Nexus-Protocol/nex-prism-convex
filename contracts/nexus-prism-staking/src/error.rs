use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("impossible: {0}")]
    Impossible(String),

    #[error("overflow: {source}")]
    Overflow {
        source: OverflowError,
        #[cfg(feature = "backtraces")]
        backtrace: Backtrace,
    },

    #[error("no stakers")]
    NoStakers {},

    #[error("no rewards")]
    NoRewards {},

    #[error("not enough {name} tokens: {value}, but {required} required")]
    NotEnoughTokens {
        name: String,
        value: Uint128,
        required: Uint128,
    },
}

impl ContractError {
    pub fn overflow(source: OverflowError) -> Self {
        ContractError::Overflow {
            source,
            #[cfg(feature = "backtraces")]
            backtrace: Backtrace::capture(),
        }
    }
}

impl From<OverflowError> for ContractError {
    fn from(source: OverflowError) -> Self {
        Self::overflow(source)
    }
}

impl From<ContractError> for StdError {
    fn from(e: ContractError) -> Self {
        StdError::generic_err(e.to_string())
    }
}
