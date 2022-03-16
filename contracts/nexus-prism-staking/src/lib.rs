mod commands;
pub mod contract;
mod error;
mod math;
mod queries;
pub mod state;
mod utils;

#[cfg(test)]
mod tests;

type ContractResult<T> = Result<T, error::ContractError>;
