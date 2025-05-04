#![deny(clippy::unwrap_used)]
#![deny(unused_must_use)]

use anchor_lang::prelude::*;

mod config;
mod instructions;
mod state;
mod utils;
use instructions::*;

declare_id!("6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2");

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;
#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Pyra",
    project_url: "https://pyra.fi/",
    contacts: "email:iarla@pyra.fi",
    policy: "https://github.com/pyra-fi/pyra-protocol/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/pyra-fi/pyra-protocol"
}

#[program]
pub mod pyra {
    use super::*;

    // Admin

    pub fn reclaim_bridge_rent(
        ctx: Context<ReclaimBridgeRent>,
        attestation: Vec<u8>,
    ) -> Result<()> {
        reclaim_bridge_rent_handler(ctx, attestation)
    }

    // User

    pub fn init_user(
        ctx: Context<InitUser>,
        spend_limit_per_transaction: u64,
        spend_limit_per_timeframe: u64,
        timeframe_in_seconds: u64,
        next_timeframe_reset_timestamp: u64,
    ) -> Result<()> {
        init_user_handler(
            ctx,
            spend_limit_per_transaction,
            spend_limit_per_timeframe,
            timeframe_in_seconds,
            next_timeframe_reset_timestamp,
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
        next_timeframe_reset_timestamp: u64,
    ) -> Result<()> {
        upgrade_vault_handler(
            ctx,
            spend_limit_per_transaction,
            spend_limit_per_timeframe,
            timeframe_in_seconds,
            next_timeframe_reset_timestamp,
        )
    }

    // Balance

    pub fn fulfil_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, FulfilDeposit<'info>>,
        drift_market_index: u16,
    ) -> Result<()> {
        fulfil_deposit_handler(ctx, drift_market_index)
    }

    pub fn rescue_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, RescueDeposit<'info>>,
    ) -> Result<()> {
        rescue_deposit_handler(ctx)
    }

    pub fn initiate_withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, InitiateWithdraw<'info>>,
        amount_base_units: u64,
        drift_market_index: u16,
        reduce_only: bool,
    ) -> Result<()> {
        initiate_withdraw_handler(ctx, amount_base_units, drift_market_index, reduce_only)
    }

    pub fn fulfil_withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, FulfilWithdraw<'info>>,
    ) -> Result<()> {
        fulfil_withdraw_handler(ctx)
    }

    pub fn cancel_withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelWithdraw<'info>>,
    ) -> Result<()> {
        cancel_withdraw_handler(ctx)
    }

    // Spend

    pub fn start_spend<'info>(
        ctx: Context<'_, '_, 'info, 'info, StartSpend<'info>>,
        amount_usdc_base_units: u64,
        spend_fee: bool,
    ) -> Result<()> {
        start_spend_handler(ctx, amount_usdc_base_units, spend_fee)
    }

    pub fn complete_spend<'info>(
        ctx: Context<'_, '_, 'info, 'info, CompleteSpend<'info>>,
    ) -> Result<()> {
        complete_spend_handler(ctx)
    }

    pub fn initiate_spend_limits<'info>(
        ctx: Context<'_, '_, 'info, 'info, InitiateSpendLimits<'info>>,
        spend_limit_per_transaction: u64,
        spend_limit_per_timeframe: u64,
        timeframe_in_seconds: u64,
        next_timeframe_reset_timestamp: u64,
    ) -> Result<()> {
        initiate_spend_limits_handler(
            ctx,
            spend_limit_per_transaction,
            spend_limit_per_timeframe,
            timeframe_in_seconds,
            next_timeframe_reset_timestamp,
        )
    }

    pub fn fulfil_spend_limits<'info>(
        ctx: Context<'_, '_, 'info, 'info, FulfilSpendLimits<'info>>,
    ) -> Result<()> {
        fulfil_spend_limits_handler(ctx)
    }

    pub fn increase_spend_limits<'info>(
        ctx: Context<'_, '_, 'info, 'info, IncreaseSpendLimits<'info>>,
        spend_limit_per_transaction: u64,
        spend_limit_per_timeframe: u64,
        timeframe_in_seconds: u64,
        next_timeframe_reset_timestamp: u64,
    ) -> Result<()> {
        increase_spend_limits_handler(
            ctx,
            spend_limit_per_transaction,
            spend_limit_per_timeframe,
            timeframe_in_seconds,
            next_timeframe_reset_timestamp,
        )
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
        deposit_collateral_repay_handler(ctx, deposit_market_index)
    }

    pub fn withdraw_collateral_repay<'info>(
        ctx: Context<'_, '_, 'info, 'info, WithdrawCollateralRepay<'info>>,
        withdraw_market_index: u16,
    ) -> Result<()> {
        withdraw_collateral_repay_handler(ctx, withdraw_market_index)
    }
}
