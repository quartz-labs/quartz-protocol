use anchor_lang::prelude::*;
use solana_program::{
    native_token::LAMPORTS_PER_SOL, 
    pubkey
};
use crate::state::DriftMarket;

pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

pub const PYTH_FEED_SOL_USD: &str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
pub const PYTH_FEED_USDC_USD: &str = "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a";

pub const DRIFT_MARKETS: [DriftMarket; 2] = [
    DriftMarket {
        market_index: 0,
        mint: USDC_MINT,
        pyth_feed: PYTH_FEED_USDC_USD,
        pyth_max_age_seconds: 60,
        base_units_per_token: 1_000_000
    },
    DriftMarket {
        market_index: 1,
        mint: WSOL_MINT,
        pyth_feed: PYTH_FEED_SOL_USD,
        pyth_max_age_seconds: 30,
        base_units_per_token: LAMPORTS_PER_SOL
    }
];