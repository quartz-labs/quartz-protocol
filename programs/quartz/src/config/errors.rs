use anchor_lang::prelude::*;

#[error_code]
pub enum QuartzError {
    #[msg("Vault already initialized")]
    VaultAlreadyInitialized,
    #[msg("Illegal collateral repay instructions")]
    IllegalCollateralRepayInstructions,
    #[msg("Invalid mint provided")]
    InvalidMint,
    #[msg("Price slippage is above maximum")]
    MaxSlippageExceeded,
    #[msg("Swap platform fee must be zero")]
    InvalidPlatformFee,
    #[msg("User accounts accross instructions must match")]
    InvalidUserAccounts,
    #[msg("Swap source token account does not match withdraw")]
    InvalidSourceTokenAccount,
    #[msg("Swap destination token account does not match deposit")]
    InvalidDestinationTokenAccount,
    #[msg("Declared start balance is not accurate")]
    InvalidStartBalance,
    #[msg("Price received from oracle should be a positive number")]
    NegativeOraclePrice,
    #[msg("Invalid market index")]
    InvalidMarketIndex,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Price exponents received from oracle should be the same")]
    InvalidPriceExponent,
    #[msg("Unable to load account loader")]
    UnableToLoadAccountLoader,
    #[msg("Could not deserialize introspection instruction data")]
    DeserializationError,
    #[msg("Total collateral cannot be less than margin requirement for auto repay")]
    AutoRepayThresholdNotReached,
    #[msg("Too much collateral sold in auto repay")]
    AutoRepayTooMuchSold,
    #[msg("Not enough collateral sold in auto repay")]
    AutoRepayNotEnoughSold,
    #[msg("Collateral repay deposit and withdraw markets must be different")]
    IdenticalCollateralRepayMarkets,
    #[msg("Invalid starting vault balance")]
    InvalidStartingVaultBalance,
    #[msg("Provided token ledger is not empty")]
    FreshTokenLedgerRequired,
    #[msg("Provided EVM address does not match expected format")]
    InvalidEvmAddress,
    #[msg("Invalid vault owner")]
    InvalidVaultOwner,
    #[msg("Invalid vault address")]
    InvalidVaultAddress,
    #[msg("Lookup table already initialized")]
    LookupTableAlreadyInitialized,
    #[msg("Missing token mint")]
    MissingTokenMint,
    #[msg("Invalid token program id")]
    InvalidTokenProgramId,
    #[msg("Invalid lookup table")]
    InvalidLookupTable,
    #[msg("Invalid lookup table content")]
    InvalidLookupTableContent,
    #[msg("Invalid lookup table authority")]
    InvalidLookupTableAuthority,
    #[msg("Insufficient spend limit remaining for the timeframe")]
    InsufficientTimeframeSpendLimit,
    #[msg("Transaction is larger than the transaction spend limit")]
    InsufficientTransactionSpendLimit,
    #[msg("start_spend instruction must be followed by complete_spend instruction")]
    IllegalSpendInstructions,
    #[msg("Current timestamp cannot be negative")]
    InvalidTimestamp,
    #[msg("Time lock rent payer must either be the owner or the time_lock_rent_payer PDA")]
    InvalidTimeLockRentPayer,
    #[msg("Release slot has not passed for time lock")]
    TimeLockNotReleased,
    #[msg("Time lock owner does not match")]
    InvalidTimeLockOwner,
    #[msg("An initialize instruction was sent to an account that has already been initialized")]
    AccountAlreadyInitialized,
    #[msg("owner_spl is required if spl_mint is not wSOL")]
    InvalidOwnerSplWSOL,
}
