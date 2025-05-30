use anchor_lang::prelude::*;

#[error_code]
pub enum PyraError {
    #[msg("Illegal collateral repay instructions")]
    IllegalCollateralRepayInstructions,
    #[msg("Collateral repay cannot be called as a CPI")]
    IllegalCollateralRepayCPI,
    #[msg("Invalid mint provided")]
    InvalidMint,
    #[msg("Price slippage is above maximum")]
    MaxSlippageExceeded,
    #[msg("Max slippage config is above maximum BPS")]
    InvalidSlippageBPS,
    #[msg("User accounts accross instructions must match")]
    InvalidUserAccounts,
    #[msg("Swap source token account does not match withdraw")]
    InvalidSourceTokenAccount,
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
    #[msg("Provided EVM address does not match expected format")]
    InvalidEvmAddress,
    #[msg("Invalid vault owner")]
    InvalidVaultOwner,
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
    #[msg("destination_spl is required if spl_mint is not wSOL")]
    MissingDestinationSpl,
    #[msg("deposit_address_spl is required if spl_mint is not wSOL")]
    MissingDepositAddressSpl,
    #[msg("Withdraw destination does not match order account")]
    InvalidWithdrawDestination,
    #[msg("Invalid spend fee destination")]
    InvalidSpendFeeDestination,
    #[msg("Invalid spend caller")]
    InvalidSpendCaller,
    #[msg("Account is already initialized")]
    AccountAlreadyInitialized,
    #[msg("Invalid rent reclaimer")]
    InvalidRentReclaimer,
    #[msg("Failed to deserialize market index")]
    FailedToDeserializeMarketIndex,
    #[msg("Failed to deserialize vault bytes")]
    FailedToDeserializeVaultBytes,
    #[msg("Invalid vault account")]
    InvalidVaultAccount,
    #[msg("Vault data was illegally modified during a CPI")]
    IllegalVaultCPIModification,
    #[msg("Deposit address must be owned by the system program")]
    InvalidDepositAddressOwner,
    #[msg("Spend fee BPS is above maximum")]
    InvalidSpendFeeBPS,
    #[msg("Invalid USDC ATA for deposit address")]
    InvalidDepositAddressUSDC,
    #[msg("Spend limit cannot be decreased in the increase spend limits instruction")]
    IllegalSpendLimitDecrease,
    #[msg("Cannot rescue a supported token")]
    IllegalRescueSupportedToken,
    #[msg("Cannot transfer zero tokens")]
    TransferZero,
}
