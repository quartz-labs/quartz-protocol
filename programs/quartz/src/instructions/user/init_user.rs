use anchor_lang::prelude::*;
use crate::state::Vault;

#[derive(Accounts)]
pub struct InitializeUser<'info> {
    #[account(
        init,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump,
        payer = owner,
        space = Vault::INIT_SPACE
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: This is a lookup table account that will be validated by the migrate_vault instruction
    pub lookup_table: UncheckedAccount<'info>,
}

pub fn init_user_handler(ctx: Context<InitializeUser>, spend_balance_amount: u64) -> Result<()> {
    ctx.accounts.vault.owner = ctx.accounts.owner.key();
    ctx.accounts.vault.bump = ctx.bumps.vault;
    ctx.accounts.vault.spend_balance_amount = spend_balance_amount;

    // TODO: Create a lookup table through the system program
    ctx.accounts.vault.lookup_table = ctx.accounts.lookup_table.key();
    Ok(())
}
