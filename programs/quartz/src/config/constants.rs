use anchor_lang::prelude::*;
use solana_program::pubkey;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U8_SIZE: usize = 1;
pub const U16_SIZE: usize = 2;
pub const U64_SIZE: usize = 8;

pub const JUPITER_ID: Pubkey = pubkey!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
pub const JUPITER_EXACT_OUT_ROUTE_DISCRIMINATOR: [u8; 8] = [208, 51, 239, 151, 123, 43, 237, 92];
pub const JUPITER_EXACT_IN_ROUTE_DISCRIMINATOR: [u8; 8] = [229, 23, 203, 203, 122, 227, 173, 42];
pub const JUPITER_SWAP_DISCRIMINATORS: &[[u8; 8]] = &[
    JUPITER_EXACT_OUT_ROUTE_DISCRIMINATOR,
    JUPITER_EXACT_IN_ROUTE_DISCRIMINATOR
];

pub const ACCOUNT_HEALTH_BUFFER_PERCENT: u8 = 10;
pub const COLLATERAL_REPAY_MAX_HEALTH_RESULT_PERCENT: u8 = 30;
pub const COLLATERAL_REPAY_MAX_SLIPPAGE_BPS: u16 = 100;
pub const PYTH_MAX_PRICE_AGE_SECONDS: u64 = 60;
