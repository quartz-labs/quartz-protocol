use anchor_lang::prelude::*;
use solana_address_lookup_table_program::state::LOOKUP_TABLE_META_SIZE;
use solana_program::pubkey;

use super::DRIFT_MARKETS;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U8_SIZE: usize = 1;
pub const U64_SIZE: usize = 8;

pub const USER_ADDRES_LOOKUP_TABLE_DEFAULT_ACCOUNTS: u64 = 2;
pub const USER_ADDRES_LOOKUP_TABLE_SIZE: u64 = 
    LOOKUP_TABLE_META_SIZE as u64 +
    (DRIFT_MARKETS.len() * PUBKEY_SIZE * 2) as u64;

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

pub const DRIFT_ID: Pubkey = pubkey!("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH");
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ADDRESS_LOOKUP_TABLE_PROGRAM_ID: Pubkey = pubkey!("AddressLookupTab1e1111111111111111111111111");
