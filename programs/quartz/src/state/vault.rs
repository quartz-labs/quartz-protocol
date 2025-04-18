use crate::config::{ANCHOR_DISCRIMINATOR, PUBKEY_SIZE, U64_SIZE, U8_SIZE};
use anchor_lang::prelude::*;

#[account]
pub struct Vault {
    // Note: If the owner becomes changeable in the future, need to add has_one contstraints to all ixs
    pub owner: Pubkey,
    pub bump: u8,

    pub spend_limit_per_transaction: u64,
    pub spend_limit_per_timeframe: u64,
    pub remaining_spend_limit_per_timeframe: u64,

    // The next timestamp the remaining_spend_limit_per_timeframe will be reset at
    pub next_timeframe_reset_timestamp: u64,

    // How much to extend the next_timeframe_reset_timestamp by when it's reached
    pub timeframe_in_seconds: u64,
}

impl Space for Vault {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR
        + PUBKEY_SIZE
        + U8_SIZE
        + U64_SIZE
        + U64_SIZE
        + U64_SIZE
        + U64_SIZE
        + U64_SIZE;
}
