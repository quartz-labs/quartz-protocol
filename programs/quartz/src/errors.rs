use anchor_lang::prelude::*;

#[error_code]
pub enum QuartzError {
    #[msg("Illegal collateral repay instructions")]
    IllegalCollateralRepayInstructions,
    #[msg("Invalid mint provided")]
    InvalidRepayMint,
    #[msg("Price slippage is above maximum")]
    MaxSlippageExceeded,
    #[msg("Swap platform fee must be zero")]
    InvalidPlatformFee,
    #[msg("User accounts for deposit and withdraw do not match")]
    InvalidUserAccounts,
    #[msg("Swap source token account does not match withdraw")]
    InvalidSourceTokenAccount,
    #[msg("Swap destination token account does not match deposit")]
    InvalidDestinationTokenAccount,
    #[msg("Declared start balance is not accurate")]
    InvalidStartBalance,
    #[msg("Price received from oracle should be a positive number")]
    NegativeOraclePrice,
    #[msg("Unsupported Drift market index")]
    UnsupportedDriftMarketIndex,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Price exponents received from oracle should be the same")]
    InvalidPriceExponent,
    #[msg("Unable to load account loader")]
    UnableToLoadAccountLoader,
    #[msg("Could not deserialize introspection instruction data")]
    DeserializationError,
    #[msg("Account health is not low enough for collateral_repay")]
    NotReachedCollateralRepayThreshold,
    #[msg("Not enough collateral sold in collateral_repay")]
    CollateralRepayHealthTooLow,
    #[msg("Too much collateral sold in collateral_repay")]
    CollateralRepayHealthTooHigh,
}
