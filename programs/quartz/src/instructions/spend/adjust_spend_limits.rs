use anchor_lang::prelude::*;
use crate::state::Vault;

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
    ctx.accounts.vault.spend_limit_per_transaction = spend_limit_per_transaction;
    ctx.accounts.vault.spend_limit_per_timeframe = spend_limit_per_timeframe;
    ctx.accounts.vault.timeframe_in_slots = timeframe_in_slots;

    Ok(())
}