use anchor_lang::prelude::*;

mod state;
mod utils;
mod config;
mod instructions;
use instructions::*;

declare_id!("6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2");

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;
#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Quartz",
    project_url: "https://quartzpay.io/",
    contacts: "email:iarla@quartzpay.io",
    policy: "https://github.com/quartz-labs/quartz-protocol/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/quartz-labs/quartz-protocol"
}

#[program]
pub mod quartz {
    use super::*;

    // Admin

    pub fn reclaim_bridge_rent(
        ctx: Context<ReclaimBridgeRent>,
        attestation: Vec<u8>
    ) -> Result<()> {
        reclaim_bridge_rent_handler(
            ctx, 
            attestation
        )
    }

    // User

    pub fn init_user(
        ctx: Context<InitUser>, 
        spend_limit_per_transaction: u64,
        spend_limit_per_timeframe: u64,
        timeframe_in_seconds: u64,
        next_timeframe_reset_timestamp: u64
    ) -> Result<()> {
        init_user_handler(
            ctx, 
            spend_limit_per_transaction, 
            spend_limit_per_timeframe, 
            timeframe_in_seconds,
            next_timeframe_reset_timestamp
        )
    }

    pub fn close_user(ctx: Context<CloseUser>) -> Result<()> {
        close_user_handler(ctx)
    }

    pub fn upgrade_vault(
        ctx: Context<UpgradeVault>,
        spend_limit_per_transaction: u64,
        spend_limit_per_timeframe: u64,
        timeframe_in_seconds: u64,
        next_timeframe_reset_timestamp: u64
    ) -> Result<()> {
        upgrade_vault_handler(
            ctx, 
            spend_limit_per_transaction, 
            spend_limit_per_timeframe, 
            timeframe_in_seconds,
            next_timeframe_reset_timestamp
        )
    }

    // Balance

    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, Deposit<'info>>, 
        amount_base_units: u64, 
        drift_market_index: u16,
        reduce_only: bool
    ) -> Result<()> {
        deposit_handler(
            ctx, 
            amount_base_units, 
            drift_market_index, 
            reduce_only
        )
    }

    pub fn fulfil_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, FulfilDeposit<'info>>, 
        amount_base_units: u64,
        drift_market_index: u16,
        reduce_only: bool
    ) -> Result<()> {
        fulfil_deposit_handler(
            ctx, 
            amount_base_units, 
            drift_market_index, 
            reduce_only
        )
    }

    pub fn withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>, 
        amount_base_units: u64, 
        drift_market_index: u16,
        reduce_only: bool
    ) -> Result<()> {
        withdraw_handler(
            ctx, 
            amount_base_units, 
            drift_market_index, 
            reduce_only
        )
    }

    pub fn initiate_withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, InitiateWithdraw<'info>>,
        amount_base_units: u64,
        drift_market_index: u16,
        reduce_only: bool
    ) -> Result<()> {
        initiate_withdraw_handler(
            ctx, 
            amount_base_units, 
            drift_market_index, 
            reduce_only
        )
    }

    pub fn fulfil_withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, FulfilWithdraw<'info>>,
    ) -> Result<()> {
        fulfil_withdraw_handler(ctx)
    }

    // Spend

    pub fn start_spend<'info>(
        ctx: Context<'_, '_, 'info, 'info, StartSpend<'info>>,
        amount_usdc_base_units: u64,
        spend_fee: bool
    ) -> Result<()> {
        start_spend_handler(ctx, amount_usdc_base_units, spend_fee)
    }

    pub fn complete_spend<'info>(
        ctx: Context<'_, '_, 'info, 'info, CompleteSpend<'info>>,
    ) -> Result<()> {
        complete_spend_handler(ctx)
    }

    pub fn adjust_spend_limits<'info>(
        ctx: Context<'_, '_, 'info, 'info, AdjustSpendLimits<'info>>,
        spend_limit_per_transaction: u64,
        spend_limit_per_timeframe: u64,
        timeframe_in_seconds: u64,
        next_timeframe_reset_timestamp: u64
    ) -> Result<()> {
        adjust_spend_limits_handler(
            ctx, 
            spend_limit_per_transaction, 
            spend_limit_per_timeframe, 
            timeframe_in_seconds,
            next_timeframe_reset_timestamp
        )
    }

    pub fn initiate_spend_limits<'info>(
        ctx: Context<'_, '_, 'info, 'info, InitiateSpendLimits<'info>>,
        spend_limit_per_transaction: u64,
        spend_limit_per_timeframe: u64,
        timeframe_in_seconds: u64,
        next_timeframe_reset_timestamp: u64
    ) -> Result<()> {
        initiate_spend_limits_handler(
            ctx, 
            spend_limit_per_transaction, 
            spend_limit_per_timeframe, 
            timeframe_in_seconds, 
            next_timeframe_reset_timestamp
        )
    }

    pub fn fulfil_spend_limits<'info>(
        ctx: Context<'_, '_, 'info, 'info, FulfilSpendLimits<'info>>,
    ) -> Result<()> {
        fulfil_spend_limits_handler(ctx)
    }

    // Collateral Repay
    pub fn start_collateral_repay<'info>(
        ctx: Context<'_, '_, 'info, 'info, StartCollateralRepay<'info>>,
    ) -> Result<()> {
        start_collateral_repay_handler(ctx)
    }

    pub fn deposit_collateral_repay<'info>(
        ctx: Context<'_, '_, 'info, 'info, DepositCollateralRepay<'info>>,
        deposit_market_index: u16,
    ) -> Result<()> {
        deposit_collateral_repay_handler(
            ctx, 
            deposit_market_index
        )
    }

    pub fn withdraw_collateral_repay<'info>(
        ctx: Context<'_, '_, 'info, 'info, WithdrawCollateralRepay<'info>>,
        withdraw_market_index: u16
    ) -> Result<()> {
        withdraw_collateral_repay_handler(
            ctx, 
            withdraw_market_index
        )
    }
}
