use crate::{
    check,
    config::{QuartzError, INIT_ACCOUNT_RENT_FEE},
    state::Vault,
    utils::validate_account_fresh,
};
use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};
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

    /// CHECK: Safe once seeds are correct, deposit address is the pubkey anyone can send tokens to for deposits
    #[account(
        mut,
        seeds = [b"deposit_address".as_ref(), vault.key().as_ref()],
        bump
    )]
    pub deposit_address: UncheckedAccount<'info>,
}

/// Close user account, repaying init fee
pub fn close_user_handler(ctx: Context<CloseUser>) -> Result<()> {
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds_vault = &[b"vault", owner.as_ref(), &[vault_bump]];
    let signer_seeds_vault = &[&seeds_vault[..]];

    let vault_lamports_before_cpi = ctx.accounts.vault.to_account_info().lamports();

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

    // Check vault data to ensure it hasn't been drained by the Drift CPI
    ctx.accounts.vault.reload()?;
    let vault_lamports_after_cpi = ctx.accounts.vault.to_account_info().lamports();
    check!(
        vault_lamports_after_cpi >= vault_lamports_before_cpi,
        QuartzError::IllegalVaultCPIModification
    );

    // Close deposit address
    let deposit_address_bump = ctx.bumps.deposit_address;
    let vault = &ctx.accounts.vault.key();
    let seeds_deposit_address = &[
        b"deposit_address".as_ref(),
        vault.as_ref(),
        &[deposit_address_bump],
    ];
    let signer_seeds_deposit_address = &[&seeds_deposit_address[..]];

    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.deposit_address.to_account_info(),
                to: ctx.accounts.init_rent_payer.to_account_info(),
            },
            signer_seeds_deposit_address,
        ),
        ctx.accounts.deposit_address.lamports(),
    )?;

    validate_account_fresh(&ctx.accounts.deposit_address.to_account_info())?;

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
