use anchor_lang::prelude::*;
use crate::{config::QuartzError, state::Vault};

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
    timeframe_in_slots: u64
) -> Result<()> {
    let spend_limit_per_timeframe_already_used = ctx.accounts.vault.spend_limit_per_timeframe
        .checked_sub(ctx.accounts.vault.remaining_spend_limit_per_timeframe)
        .ok_or(QuartzError::MathOverflow)?;

    ctx.accounts.vault.remaining_spend_limit_per_timeframe = spend_limit_per_timeframe
        .checked_sub(spend_limit_per_timeframe_already_used)
        .ok_or(QuartzError::MathOverflow)?;
    
    ctx.accounts.vault.spend_limit_per_transaction = spend_limit_per_transaction;
    ctx.accounts.vault.spend_limit_per_timeframe = spend_limit_per_timeframe;
    ctx.accounts.vault.timeframe_in_slots = timeframe_in_slots;

    // TODO: Make this calendar months
    ctx.accounts.vault.next_timeframe_reset_slot = &Clock::get()?.slot + timeframe_in_slots;

    Ok(())
}