use crate::config::{ANCHOR_DISCRIMINATOR, U64_SIZE};
use crate::state::time_lock::*;

/// Time locked order for updating the spend limits of a vault
#[account]
pub struct SpendLimitsOrder {
    pub time_lock: TimeLock,
    pub spend_limit_per_transaction: u64,
    pub spend_limit_per_timeframe: u64,
    pub timeframe_in_seconds: u64,
    pub next_timeframe_reset_timestamp: u64,
}

impl Space for SpendLimitsOrder {
    const INIT_SPACE: usize =
        ANCHOR_DISCRIMINATOR + TimeLock::INIT_SPACE + U64_SIZE + U64_SIZE + U64_SIZE + U64_SIZE;
}

impl TimeLocked for SpendLimitsOrder {
    fn time_lock(&self) -> &TimeLock {
        &self.time_lock
    }
}
