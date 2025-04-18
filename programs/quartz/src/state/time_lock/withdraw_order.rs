use crate::config::{ANCHOR_DISCRIMINATOR, PUBKEY_SIZE, U16_SIZE, U1_SIZE, U64_SIZE};
use crate::state::time_lock::*;

/// Time locked order for withdrawing funds from a vault
#[account]
pub struct WithdrawOrder {
    pub time_lock: TimeLock,
    pub amount_base_units: u64,
    pub drift_market_index: u16,
    pub reduce_only: bool,
    pub destination: Pubkey,
}

impl Space for WithdrawOrder {
    const INIT_SPACE: usize =
        ANCHOR_DISCRIMINATOR + TimeLock::INIT_SPACE + U64_SIZE + U16_SIZE + U1_SIZE + PUBKEY_SIZE;
}

impl TimeLocked for WithdrawOrder {
    fn time_lock(&self) -> &TimeLock {
        &self.time_lock
    }
}
