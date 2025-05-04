use crate::{
    check,
    config::{PyraError, DRIFT_MARKETS},
    state::Vault,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

#[derive(Accounts)]
pub struct RescueDeposit<'info> {
    #[account(
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program
    )]
    pub owner_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Safe once seeds are correct, deposit address is the pubkey anyone can send tokens to for deposits
    #[account(
        mut,
        seeds = [b"deposit_address".as_ref(), vault.key().as_ref()],
        bump
    )]
    pub deposit_address: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = deposit_address,
        associated_token::token_program = token_program
    )]
    pub deposit_address_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

/// Sends unsupported tokens from the deposit address to the destination (in case the user accidentally sends the wrong token)
pub fn rescue_deposit_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, RescueDeposit<'info>>,
) -> Result<()> {
    // Validate SPL token is not supported

    if DRIFT_MARKETS
        .iter()
        .any(|market| market.mint == ctx.accounts.mint.key())
    {
        return Err(PyraError::IllegalRescueSupportedToken.into());
    }

    // Transfer tokens from deposit address ATA to owner ATA

    let balance = ctx.accounts.deposit_address_spl.amount;
    check!(balance > 0, PyraError::TransferZero);

    let deposit_address_bump = ctx.bumps.deposit_address;
    let vault = ctx.accounts.vault.key();
    let seeds_deposit_address = &[b"deposit_address", vault.as_ref(), &[deposit_address_bump]];
    let deposit_address_signer = &[&seeds_deposit_address[..]];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.deposit_address_spl.to_account_info(),
                to: ctx.accounts.owner_spl.to_account_info(),
                authority: ctx.accounts.deposit_address.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            deposit_address_signer,
        ),
        balance,
        ctx.accounts.mint.decimals,
    )?;

    // Close deposit address ATA

    close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.deposit_address_spl.to_account_info(),
            destination: ctx.accounts.owner.to_account_info(),
            authority: ctx.accounts.deposit_address.to_account_info(),
        },
        deposit_address_signer,
    ))?;

    Ok(())
}
