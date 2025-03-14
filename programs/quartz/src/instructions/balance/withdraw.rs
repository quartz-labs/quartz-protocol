use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        TransferChecked,
        transfer_checked,
        TokenInterface, 
        TokenAccount, 
        Mint,
        CloseAccount,
        close_account
    }
};
use drift::{
    program::Drift,
    cpi::withdraw as drift_withdraw, 
    cpi::accounts::Withdraw as DriftWithdraw,
    state::{
        state::State as DriftState, 
        user::{User as DriftUser, UserStats as DriftUserStats}
    }
};
use crate::{
    check, config::QuartzError, state::Vault, utils::get_drift_market
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        seeds = [vault.key().as_ref(), spl_mint.key().as_ref()],
        bump,
        payer = owner,
        token::mint = spl_mint,
        token::authority = vault
    )]
    pub vault_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = spl_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program
    )]
    pub owner_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    pub spl_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [b"user".as_ref(), vault.key().as_ref(), (0u16).to_le_bytes().as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_user: AccountLoader<'info, DriftUser>,
    
    #[account(
        mut,
        seeds = [b"user_stats".as_ref(), vault.key().as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_user_stats: AccountLoader<'info, DriftUserStats>,

    #[account(
        mut,
        seeds = [b"drift_state".as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_state: Box<Account<'info, DriftState>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,
    
    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    pub drift_signer: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,
}

pub fn withdraw_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>, 
    amount_base_units: u64,
    drift_market_index: u16,
    reduce_only: bool
) -> Result<()> {
    // Validate market index and mint
    let drift_market = get_drift_market(drift_market_index)?;
    check!(
        &ctx.accounts.spl_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );
    
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    // Paranoia check to ensure the vault is empty before withdrawing for amount calculations
    check!(
        ctx.accounts.vault_spl.amount == 0,
        QuartzError::InvalidStartingVaultBalance
    );

    // Drift Withdraw CPI
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        DriftWithdraw {
            state: ctx.accounts.drift_state.to_account_info(),
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            spot_market_vault: ctx.accounts.spot_market_vault.to_account_info(),
            drift_signer: ctx.accounts.drift_signer.to_account_info(),
            user_token_account: ctx.accounts.vault_spl.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    drift_withdraw(cpi_ctx, drift_market_index, amount_base_units, reduce_only)?;

    // Transfer tokens to owner's ATA, getting the true amount withdrawn (in case return_only prevented full withdraw)
    ctx.accounts.vault_spl.reload()?;
    let true_amount_withdrawn = ctx.accounts.vault_spl.amount;
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            TransferChecked { 
                from: ctx.accounts.vault_spl.to_account_info(), 
                to: ctx.accounts.owner_spl.to_account_info(), 
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.spl_mint.to_account_info(),
            }, 
            signer_seeds
        ),
        true_amount_withdrawn,
        ctx.accounts.spl_mint.decimals
    )?;

    // Close vault's ATA
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault_spl.to_account_info(),
            destination: ctx.accounts.owner.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds
    );
    close_account(cpi_ctx_close)?;

    Ok(())
}