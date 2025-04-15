use crate::{
    config::{ANCHOR_DISCRIMINATOR, PUBKEY_SIZE, U16_SIZE, U1_SIZE, U64_SIZE, U8_SIZE},
    utils::{TimeLock, TimeLocked},
};
use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

pub struct DriftMarket {
    pub market_index: u16,
    pub mint: Pubkey,
    pub pyth_feed: &'static str,
    pub base_units_per_token: u64,
}

#[account]
pub struct Vault {
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

#[account]
pub struct CollateralRepayLedger {
    pub deposit: u64,
    pub withdraw: u64,
}

impl Space for CollateralRepayLedger {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR + U64_SIZE + U64_SIZE;
}

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
