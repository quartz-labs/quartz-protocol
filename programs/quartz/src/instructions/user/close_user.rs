use crate::{config::INIT_ACCOUNT_RENT_FEE, state::Vault};
use anchor_lang::prelude::*;
use drift::{
    cpi::{accounts::DeleteUser, delete_user},
    program::Drift,
};
use solana_program::{program::invoke_signed, system_instruction};

#[derive(Accounts)]
pub struct CloseUser<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        close = init_rent_payer
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Safe once address is correct
    #[account(
        mut,
        seeds = [b"init_rent_payer"],
        bump
    )]
    pub init_rent_payer: UncheckedAccount<'info>,

    /// CHECK: Passed into Drift CPI (which performs the security checks)
    #[account(mut)]
    pub drift_user: UncheckedAccount<'info>,

    /// CHECK: Passed into Drift CPI (which performs the security checks)
    #[account(mut)]
    pub drift_user_stats: UncheckedAccount<'info>,

    /// CHECK: Passed into Drift CPI (which performs the security checks)
    #[account(mut)]
    pub drift_state: UncheckedAccount<'info>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,
}

pub fn close_user_handler(ctx: Context<CloseUser>) -> Result<()> {
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds_vault = &[b"vault", owner.as_ref(), &[vault_bump]];
    let signer_seeds_vault = &[&seeds_vault[..]];

    let delete_user_cpi_context = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        DeleteUser {
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            state: ctx.accounts.drift_state.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds_vault,
    );

    delete_user(delete_user_cpi_context)?;

    // Repay user the init rent fee
    let init_rent_payer_bump = ctx.bumps.init_rent_payer;
    let seeds_init_rent_payer = &[b"init_rent_payer".as_ref(), &[init_rent_payer_bump]];
    let signer_seeds_init_rent_payer = &[&seeds_init_rent_payer[..]];

    invoke_signed(
        &system_instruction::transfer(
            ctx.accounts.init_rent_payer.key,
            ctx.accounts.owner.key,
            INIT_ACCOUNT_RENT_FEE,
        ),
        &[
            ctx.accounts.init_rent_payer.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds_init_rent_payer,
    )?;

    Ok(())
}
