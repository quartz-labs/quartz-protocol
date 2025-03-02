use anchor_lang::prelude::*;
use crate::{events::{CommonFields, SpendLimitUpdatedEvent}, state::Vault};

#[event_cpi]
#[derive(Accounts)]
pub struct AdjustSpendLimits<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        has_one = owner,
        bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(mut)]
    pub owner: Signer<'info>
}

pub fn adjust_spend_limits_handler(
    ctx: Context<AdjustSpendLimits>,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64
) -> Result<()> {
    let spend_limit_per_timeframe_already_used = ctx.accounts.vault.spend_limit_per_timeframe
        .checked_sub(ctx.accounts.vault.remaining_spend_limit_per_timeframe)
        .unwrap_or(0);

    ctx.accounts.vault.remaining_spend_limit_per_timeframe = spend_limit_per_timeframe
        .checked_sub(spend_limit_per_timeframe_already_used)
        .unwrap_or(0);
    
    ctx.accounts.vault.spend_limit_per_transaction = spend_limit_per_transaction;
    ctx.accounts.vault.spend_limit_per_timeframe = spend_limit_per_timeframe;
    ctx.accounts.vault.timeframe_in_seconds = timeframe_in_seconds;
    ctx.accounts.vault.next_timeframe_reset_timestamp = next_timeframe_reset_timestamp;

    let clock = Clock::get()?;
    emit_cpi!(SpendLimitUpdatedEvent {
        common_fields: CommonFields::new(&clock, ctx.accounts.owner.key()),
        spend_limit_per_transaction: ctx.accounts.vault.spend_limit_per_transaction,
        spend_limit_per_timeframe: ctx.accounts.vault.spend_limit_per_timeframe,
        remaining_spend_limit_per_timeframe: ctx.accounts.vault.remaining_spend_limit_per_timeframe,
        next_timeframe_reset_timestamp: ctx.accounts.vault.next_timeframe_reset_timestamp,
        timeframe_in_seconds: ctx.accounts.vault.timeframe_in_seconds
    });

    Ok(())
}