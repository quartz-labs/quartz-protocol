use anchor_lang::prelude::*;
use crate::{
    config::RENT_RECLAIMER, 
    state::Vault
};

#[derive(Accounts)]
pub struct ResetSpendLimit<'info> {
    #[account(
        constraint = rent_reclaimer.key().eq(&RENT_RECLAIMER)
    )]
    pub rent_reclaimer: Signer<'info>,

    #[account(mut)]
    pub vault1: Account<'info, Vault>,

    #[account(mut)]
    pub vault2: Account<'info, Vault>,

    #[account(mut)]
    pub vault3: Account<'info, Vault>,

    #[account(mut)]
    pub vault4: Account<'info, Vault>,

    #[account(mut)]
    pub vault5: Account<'info, Vault>,

    #[account(mut)]
    pub vault6: Account<'info, Vault>,    
}


pub fn reset_spend_limit_handler<'info>(
    ctx: Context<ResetSpendLimit<'info>>,
) -> Result<()> {
    ctx.accounts.vault1.spend_limit_per_transaction = 1000_000_000;
    ctx.accounts.vault1.spend_limit_per_timeframe = 0;
    ctx.accounts.vault1.timeframe_in_slots = (1_000 * 60 * 60 * 24) / 400;

    ctx.accounts.vault2.spend_limit_per_transaction = 1000_000_000;
    ctx.accounts.vault2.spend_limit_per_timeframe = 0;
    ctx.accounts.vault2.timeframe_in_slots = (1_000 * 60 * 60 * 24) / 400;

    ctx.accounts.vault3.spend_limit_per_transaction = 1000_000_000;
    ctx.accounts.vault3.spend_limit_per_timeframe = 0;
    ctx.accounts.vault3.timeframe_in_slots = (1_000 * 60 * 60 * 24) / 400;

    ctx.accounts.vault4.spend_limit_per_transaction = 1000_000_000;
    ctx.accounts.vault4.spend_limit_per_timeframe = 0;
    ctx.accounts.vault4.timeframe_in_slots = (1_000 * 60 * 60 * 24) / 400;

    ctx.accounts.vault5.spend_limit_per_transaction = 1000_000_000;
    ctx.accounts.vault5.spend_limit_per_timeframe = 0;
    ctx.accounts.vault5.timeframe_in_slots = (1_000 * 60 * 60 * 24) / 400;

    ctx.accounts.vault6.spend_limit_per_transaction = 1000_000_000;
    ctx.accounts.vault6.spend_limit_per_timeframe = 0;
    ctx.accounts.vault6.timeframe_in_slots = (1_000 * 60 * 60 * 24) / 400;

    Ok(())
}