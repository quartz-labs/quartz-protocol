use solana_program::{
    native_token::LAMPORTS_PER_SOL, 
    pubkey
};
use crate::state::DriftMarket;

pub const DRIFT_MARKETS: [DriftMarket; 5] = [
    DriftMarket { // WSOL
        market_index: 1,
        mint: pubkey!("So11111111111111111111111111111111111111112"),
        pyth_feed: "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d",
        base_units_per_token: LAMPORTS_PER_SOL
    },
    DriftMarket { // USDC
        market_index: 0,
        mint: pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        pyth_feed: "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a",
        base_units_per_token: 1_000_000
    },
    DriftMarket { // USDT
        market_index: 5,
        mint: pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
        pyth_feed: "0x2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b",
        base_units_per_token: 1_000_000
    },
    DriftMarket { // PYUSD
        market_index: 22,
        mint: pubkey!("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo"),
        pyth_feed: "0xc1da1b73d7f01e7ddd54b3766cf7fcd644395ad14f70aa706ec5384c59e76692",
        base_units_per_token: 1_000_000
    },
    DriftMarket { // USDS
        market_index: 28,
        mint: pubkey!("USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA"),
        pyth_feed: "0x77f0971af11cc8bac224917275c1bf55f2319ed5c654a1ca955c82fa2d297ea1",
        base_units_per_token: 1_000_000
    }
];
