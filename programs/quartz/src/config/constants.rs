use anchor_lang::prelude::*;
use solana_program::pubkey;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U8_SIZE: usize = 1;
pub const U64_SIZE: usize = 8;

pub const INIT_ACCOUNT_RENT_FEE: u64 = 0_035_000_000;

pub const AUTO_REPAY_MAX_HEALTH_RESULT_PERCENT: u8 = 25;
pub const AUTO_REPAY_MAX_SLIPPAGE_BPS: u16 = 100;
pub const PYTH_MAX_PRICE_AGE_SECONDS: u64 = 60;

pub const DOMAIN_BASE: u32 = 6;
pub const USDC_MARKET_INDEX: u16 = 0;
pub const PROVIDER_BASE_ADDRESS: &str = "0x55a2eeB9028ee51Ef91352Fa9f84A9450C5Af099";
pub const QUARTZ_CALLER_BASE_ADDRESS: &str = "0x28A0105A0cf8C0485a4956ba14b5274e9ED229DE";
pub const RENT_RECLAIMER: Pubkey = pubkey!("AhLjdeYqv4Ytw5sukK4z3x37ZGaSJ44pRqdcxqHP4ChS");
pub const SPEND_CALLER: Pubkey = pubkey!("JDd7PJDZJ8kwwzJpvUZ5qp9kXYAr9YdAEEVUNE1pFqhP");
pub const SPEND_FEE_DESTINATION: Pubkey = pubkey!("9n8AU9ErEN4SrNgpVEuWSZFrHzB55sEx3UDJYk7KcxSm");

pub const SPEND_FEE_BPS: u64 = 50;

pub const DRIFT_DELETE_USER_DISCRIMINATOR: &[u8] = &[186, 85, 17, 249, 219, 231, 98, 251];
