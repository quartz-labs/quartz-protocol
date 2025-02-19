use anchor_lang::prelude::*;
use crate::config::{ANCHOR_DISCRIMINATOR, PUBKEY_SIZE, U8_SIZE, U64_SIZE};
use solana_program::pubkey::Pubkey;

pub struct DriftMarket {
    pub market_index: u16,
    pub mint: Pubkey,
    pub pyth_feed: &'static str,
    pub base_units_per_token: u64
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub bump: u8,
    
    pub spend_limit_per_transaction: u64,
    pub spend_limit_per_timeframe: u64,
    pub remaining_spend_limit_per_timeframe: u64,

    // The next slot the remaining_spend_limit_per_timeframe will be reset at
    pub next_timeframe_reset_slot: u64, 

    // How much to extend the next_timeframe_reset_slot by when it's reached
    pub timeframe_in_slots: u64 
}

impl Space for Vault {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR 
        + PUBKEY_SIZE + U8_SIZE 
        + U64_SIZE + U64_SIZE  + U64_SIZE 
        + U64_SIZE 
        + U64_SIZE;
}

#[account]
pub struct CollateralRepayLedger {
    pub deposit: u64,
    pub withdraw: u64
}

impl Space for CollateralRepayLedger {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR 
        + U64_SIZE + U64_SIZE;
}
