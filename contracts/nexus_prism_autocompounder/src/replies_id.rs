use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum ReplyId {
    AutoCompoundingTokenCreated,
    RewardsClaimed,
    RewardsSold,
}
