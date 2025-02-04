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

    // Config

    pub fn init_user(ctx: Context<InitializeUser>) -> Result<()> {
        init_user_handler(ctx)
    }

    pub fn close_user(ctx: Context<CloseUser>) -> Result<()> {
        close_user_handler(ctx)
    }

    pub fn init_drift_account(ctx: Context<InitDriftAccount>) -> Result<()> {
        init_drift_account_handler(ctx)
    }

    pub fn close_drift_account(ctx: Context<CloseDriftAccount>) -> Result<()> {
        close_drift_account_handler(ctx)
    }

    // User

    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, Deposit<'info>>, 
        amount_base_units: u64, 
        drift_market_index: u16,
        reduce_only: bool
    ) -> Result<()> {
        deposit_handler(ctx, amount_base_units, drift_market_index, reduce_only)
    }

    pub fn withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>, 
        amount_base_units: u64, 
        drift_market_index: u16,
        reduce_only: bool
    ) -> Result<()> {
        withdraw_handler(ctx, amount_base_units, drift_market_index, reduce_only)
    }

    pub fn topup_card<'info>(
        ctx: Context<'_, '_, '_, 'info, TopupCard<'info>>,
        amount_usdc_base_units: u64,
    ) -> Result<()> {
        topup_card_handler(ctx, amount_usdc_base_units)
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
        withdraw_market_index: u16
    ) -> Result<()> {
        withdraw_collateral_repay_handler(ctx, withdraw_market_index)
    }
}
