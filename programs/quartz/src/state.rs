use anchor_lang::prelude::*;
use crate::config::{ANCHOR_DISCRIMINATOR, PUBKEY_SIZE, U8_SIZE, U64_SIZE};

pub struct DriftMarket {
    pub market_index: u16,
    pub mint: Pubkey,
    pub pyth_feed: &'static str,
    pub base_units_per_token: u64
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub bump: u8
}

impl Space for Vault {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR + PUBKEY_SIZE + U8_SIZE;
}

#[account]
pub struct CollateralRepayLedger {
    pub deposit: u64,
    pub withdraw: u64
}

impl Space for CollateralRepayLedger {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR + U64_SIZE + U64_SIZE;
}