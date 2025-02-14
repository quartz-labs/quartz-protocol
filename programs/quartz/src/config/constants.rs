use anchor_lang::prelude::*;
use solana_program::pubkey;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U8_SIZE: usize = 1;
pub const U64_SIZE: usize = 8;

pub const INIT_ACCOUNT_RENT_FEE: u64 = 0_050_000_000;

pub const ACCOUNT_HEALTH_BUFFER_PERCENT: u8 = 10;
pub const COLLATERAL_REPAY_MAX_HEALTH_RESULT_PERCENT: u8 = 30;
pub const COLLATERAL_REPAY_MAX_SLIPPAGE_BPS: u16 = 100;
pub const PYTH_MAX_PRICE_AGE_SECONDS: u64 = 60;

pub const DOMAIN_BASE: u32 = 6;
pub const USDC_MARKET_INDEX: u16 = 0;
pub const PROVIDER_BASE_ADDRESS: &str = "0x55a2eeB9028ee51Ef91352Fa9f84A9450C5Af099";
pub const QUARTZ_CALLER_BASE_ADDRESS: &str = "0x28A0105A0cf8C0485a4956ba14b5274e9ED229DE";
pub const RENT_RECLAIMER: Pubkey = pubkey!("AhLjdeYqv4Ytw5sukK4z3x37ZGaSJ44pRqdcxqHP4ChS");

pub const DRIFT_DELETE_USER_DISCRIMINATOR: &[u8] = &[186, 85, 17, 249, 219, 231, 98, 251];
pub const MARGINFI_ACCOUNT_INITIALIZE_DISCRIMINATOR: &[u8] = &[74, 115, 99, 93, 197, 69, 103, 7];
pub const MARGINFI_PROGRAM_ID: Pubkey = pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");
pub const MARGINFI_GROUP_1: Pubkey = pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");
