use crate::config::{ANCHOR_DISCRIMINATOR, U64_SIZE};
use anchor_lang::prelude::*;

/// Ledger for tracking the balance changes of each token during the swap instruction of collateral repay
#[account]
pub struct CollateralRepayLedger {
    pub deposit: u64,
    pub withdraw: u64,
}

impl Space for CollateralRepayLedger {
    const INIT_SPACE: usize = ANCHOR_DISCRIMINATOR + U64_SIZE + U64_SIZE;
}
