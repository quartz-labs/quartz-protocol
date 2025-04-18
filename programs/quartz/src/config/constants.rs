use anchor_lang::prelude::*;
use solana_program::pubkey;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U1_SIZE: usize = 1;
pub const U8_SIZE: usize = 1;
pub const U16_SIZE: usize = 2;
pub const U64_SIZE: usize = 8;
pub const DEPOSIT_ADDRESS_SPACE: usize = 0;

pub const INIT_ACCOUNT_RENT_FEE: u64 = 35_000_000; // 0.035 SOL

pub const AUTO_REPAY_MAX_HEALTH_RESULT_PERCENT: u8 = 20;
pub const AUTO_REPAY_MAX_SLIPPAGE_BPS: u16 = 100;
pub const PYTH_MAX_PRICE_AGE_SECONDS: u64 = 60;

pub const USDC_MARKET_INDEX: u16 = 0;
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

pub const DOMAIN_BASE: u32 = 6;
pub const PROVIDER_BASE_ADDRESS: &str = "0x55a2eeB9028ee51Ef91352Fa9f84A9450C5Af099";
pub const QUARTZ_CALLER_BASE_ADDRESS: &str = "0x28A0105A0cf8C0485a4956ba14b5274e9ED229DE";
pub const RENT_RECLAIMER: Pubkey = pubkey!("AhLjdeYqv4Ytw5sukK4z3x37ZGaSJ44pRqdcxqHP4ChS");
pub const SPEND_CALLER: Pubkey = pubkey!("JDd7PJDZJ8kwwzJpvUZ5qp9kXYAr9YdAEEVUNE1pFqhP");
pub const SPEND_FEE_DESTINATION: Pubkey = pubkey!("HPvsnVZQSeFr3TtD2JBjvvzxiZhnuk5MHfKRiswD4mYu");

pub const SPEND_FEE_BPS: u64 = 50;

pub const TIME_LOCK_RENT_PAYER_SEEDS: &[u8] = b"time_lock_rent_payer";
pub const TIME_LOCK_DURATION_SLOTS: u64 = 450;
