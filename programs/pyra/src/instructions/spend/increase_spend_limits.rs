use crate::{check, config::PyraError, state::Vault};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct IncreaseSpendLimits<'info> {
    #[account(
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Instantly updates the user's spend limits. No time lock is required if the spend limit is increasing.
pub fn increase_spend_limits_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, IncreaseSpendLimits<'info>>,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64,
) -> Result<()> {
    let starting_remaining_spend_limit_per_timeframe =
        ctx.accounts.vault.remaining_spend_limit_per_timeframe;
    let starting_spend_limit_per_transaction = ctx.accounts.vault.spend_limit_per_transaction;

    let spend_limit_per_timeframe_already_used = ctx
        .accounts
        .vault
        .spend_limit_per_timeframe
        .saturating_sub(ctx.accounts.vault.remaining_spend_limit_per_timeframe);

    // Set remaining limit to be the new spend limit minus what they've already used from the old limit
    // (otherwise changing anything in your spend limit would reset the remaining limit completely)
    let new_remaining_spend_limit_per_timeframe =
        spend_limit_per_timeframe.saturating_sub(spend_limit_per_timeframe_already_used);

    // Ensure spend limit has not decreased
    check!(
        new_remaining_spend_limit_per_timeframe >= starting_remaining_spend_limit_per_timeframe,
        PyraError::IllegalSpendLimitDecrease
    );
    check!(
        spend_limit_per_transaction >= starting_spend_limit_per_transaction,
        PyraError::IllegalSpendLimitDecrease
    );

    // Assign new values
    ctx.accounts.vault.remaining_spend_limit_per_timeframe =
        new_remaining_spend_limit_per_timeframe;
    ctx.accounts.vault.spend_limit_per_transaction = spend_limit_per_transaction;
    ctx.accounts.vault.spend_limit_per_timeframe = spend_limit_per_timeframe;
    ctx.accounts.vault.timeframe_in_seconds = timeframe_in_seconds;
    ctx.accounts.vault.next_timeframe_reset_timestamp = next_timeframe_reset_timestamp;

    Ok(())
}
