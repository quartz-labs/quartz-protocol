use crate::config::{ANCHOR_DISCRIMINATOR, U64_SIZE};
use anchor_lang::prelude::*;

#[account]
pub struct CollateralRepayLedger {
    pub deposit: u64,
    pub withdraw: u64,
}

impl Space for CollateralRepayLedger {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR + U64_SIZE + U64_SIZE;
}
