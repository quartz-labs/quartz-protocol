use crate::{check, config::QuartzError, state::Vault, utils::get_drift_market};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};
use drift::{
    cpi::accounts::Deposit as DriftDeposit, cpi::deposit as drift_deposit, program::Drift,
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init_if_needed,
        seeds = [vault.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = owner,
        token::mint = mint,
        token::authority = vault
    )]
    pub vault_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program
    )]
    pub owner_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_user: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_user_stats: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_state: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,
}

/// DEPRECATED: Removed in next version
pub fn deposit_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, Deposit<'info>>,
    amount_base_units: u64,
    drift_market_index: u16,
    reduce_only: bool,
) -> Result<()> {
    // Validate market index and mint
    let drift_market = get_drift_market(drift_market_index)?;
    check!(
        &ctx.accounts.mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[b"vault", owner.as_ref(), &[vault_bump]];
    let signer_seeds = &[&seeds[..]];

    // Transfer tokens from owner's ATA to vault's token account
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.owner_spl.to_account_info(),
                to: ctx.accounts.vault_spl.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        amount_base_units,
        ctx.accounts.mint.decimals,
    )?;

    // Drift Deposit CPI
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        DriftDeposit {
            state: ctx.accounts.drift_state.to_account_info(),
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            spot_market_vault: ctx.accounts.spot_market_vault.to_account_info(),
            user_token_account: ctx.accounts.vault_spl.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    drift_deposit(cpi_ctx, drift_market_index, amount_base_units, reduce_only)?;

    // Return any remaining balance (in case return_only prevented full deposit)
    ctx.accounts.vault_spl.reload()?;
    let remaining_balance = ctx.accounts.vault_spl.amount;
    if remaining_balance > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_spl.to_account_info(),
                    to: ctx.accounts.owner_spl.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                signer_seeds,
            ),
            remaining_balance,
            ctx.accounts.mint.decimals,
        )?;
    }

    // Close vault's ATA
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault_spl.to_account_info(),
            destination: ctx.accounts.owner.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds,
    );
    close_account(cpi_ctx_close)?;

    Ok(())
}
