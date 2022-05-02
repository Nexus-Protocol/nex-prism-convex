use cosmwasm_std::{StdError, Uint128};
use cw0::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("unknown reply id={id}")]
    UnknownReplyId { id: u64 },

    #[error("invalid config")]
    InvalidConfig {},

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

impl From<ContractError> for StdError {
    fn from(e: ContractError) -> Self {
        StdError::generic_err(e.to_string())
    }
}
