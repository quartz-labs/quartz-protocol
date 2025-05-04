use crate::config::{PUBKEY_SIZE, U1_SIZE, U64_SIZE};
use anchor_lang::prelude::*;

mod spend_limits_order;
pub use spend_limits_order::*;

mod withdraw_order;
pub use withdraw_order::*;

/// Time lock used to prevent an order being executed before the release_slot
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TimeLock {
    pub owner: Pubkey,
    pub is_owner_payer: bool,
    pub release_slot: u64,
}

impl Space for TimeLock {
    const INIT_SPACE: usize = PUBKEY_SIZE + U1_SIZE + U64_SIZE;
}

pub trait TimeLocked {
    fn time_lock(&self) -> &TimeLock;
}
