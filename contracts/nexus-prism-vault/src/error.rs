use cosmwasm_std::StdError;
use cw0::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("unauthorized")]
    Unauthorized {},

    #[error("invalid reward ratios")]
    InvalidRewardRatios {},

    #[error("unknown reply id={id}")]
    UnknownReply { id: u64 },
}

impl From<ContractError> for StdError {
    fn from(e: ContractError) -> Self {
        StdError::generic_err(e.to_string())
    }
}
