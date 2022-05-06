use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum ReplyId {
    NexPrismTokenCreated,
    NYLunaTokenCreated,
    NexPrismStakingCreated,
    NYLunaStakingCreated,
    PsiStakingCreated,
    NexPrismXPrismPairCreated,
    NexPrismAutocompounderCreated,
    NYLunaAutocompounderCreated,
    XPrismBoostActivated,
    VirtualRewardsClaimed,
    RealRewardsClaimed,
}
