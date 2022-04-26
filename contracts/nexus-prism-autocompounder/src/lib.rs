use cosmwasm_std::StdError;
use std::convert::TryFrom;

mod commands;
pub mod contract;
pub mod msg;
pub mod state;

#[cfg(test)]
#[allow(dead_code)]
mod tests;

pub enum SubmsgIds {
    RewardsClaimed,
    RewardsSold,
}

impl TryFrom<u64> for SubmsgIds {
    type Error = StdError;

    fn try_from(v: u64) -> Result<Self, Self::Error> {
        match v {
            x if x == SubmsgIds::RewardsClaimed.id() => Ok(SubmsgIds::RewardsClaimed),
            x if x == SubmsgIds::RewardsSold.id() => Ok(SubmsgIds::RewardsSold),
            unknown => Err(StdError::generic_err(format!(
                "unknown reply message id: {}",
                unknown
            ))),
        }
    }
}

impl SubmsgIds {
    pub const fn id(&self) -> u64 {
        match self {
            SubmsgIds::RewardsClaimed => 1,
            SubmsgIds::RewardsSold => 2,
        }
    }
}
