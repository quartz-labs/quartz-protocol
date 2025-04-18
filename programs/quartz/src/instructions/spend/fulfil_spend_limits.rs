use crate::{
    state::{SpendLimitsOrder, Vault},
    utils::{close_time_lock, validate_time_lock},
};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct FulfilSpendLimits<'info> {
    #[account(mut)]
    pub spend_limits_order: Box<Account<'info, SpendLimitsOrder>>,

    /// CHECK: Checked in handler
    #[account(mut)]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: Any account, once it has a vault (order checked in handler)
    pub owner: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn fulfil_spend_limits_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, FulfilSpendLimits<'info>>,
) -> Result<()> {
    let (
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        timeframe_in_seconds,
        next_timeframe_reset_timestamp,
    ) = get_order_data(&ctx)?;

    let spend_limit_per_timeframe_already_used = ctx
        .accounts
        .vault
        .spend_limit_per_timeframe
        .saturating_sub(ctx.accounts.vault.remaining_spend_limit_per_timeframe);

    // Set remaining limit to be the new spend limit minus what they've already used from the old limit
    // (otherwise changing anything in your spend limit would reset the remaining limit completely)
    ctx.accounts.vault.remaining_spend_limit_per_timeframe =
        spend_limit_per_timeframe.saturating_sub(spend_limit_per_timeframe_already_used);

    ctx.accounts.vault.spend_limit_per_transaction = spend_limit_per_transaction;
    ctx.accounts.vault.spend_limit_per_timeframe = spend_limit_per_timeframe;
    ctx.accounts.vault.timeframe_in_seconds = timeframe_in_seconds;
    ctx.accounts.vault.next_timeframe_reset_timestamp = next_timeframe_reset_timestamp;

    Ok(())
}

fn get_order_data<'info>(
    ctx: &Context<'_, '_, '_, 'info, FulfilSpendLimits<'info>>,
) -> Result<(u64, u64, u64, u64)> {
    validate_time_lock(
        &ctx.accounts.owner.key(),
        &ctx.accounts.spend_limits_order.time_lock,
    )?;

    let spend_limit_per_transaction = ctx.accounts.spend_limits_order.spend_limit_per_transaction;
    let spend_limit_per_timeframe = ctx.accounts.spend_limits_order.spend_limit_per_timeframe;
    let timeframe_in_seconds = ctx.accounts.spend_limits_order.timeframe_in_seconds;
    let next_timeframe_reset_timestamp = ctx
        .accounts
        .spend_limits_order
        .next_timeframe_reset_timestamp;

    close_time_lock(
        &ctx.accounts.spend_limits_order,
        &ctx.accounts.time_lock_rent_payer.to_account_info(),
    )?;

    Ok((
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        timeframe_in_seconds,
        next_timeframe_reset_timestamp,
    ))
}
