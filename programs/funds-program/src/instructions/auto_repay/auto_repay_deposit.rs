use anchor_lang::{
    prelude::*,
    solana_program::sysvar::instructions::{
        self,
        load_current_index_checked, 
        load_instruction_at_checked
    }
};
use anchor_spl::{
    associated_token::AssociatedToken, token::{self, Mint, Token, TokenAccount}
};
use drift::{
    Drift,
    cpi::deposit as drift_deposit, 
    Deposit as DriftDeposit,  
};
use crate::state::Vault;

#[derive(Accounts)]
#[instruction(
    drift_market_index: u16,
)]
pub struct AutoRepayDeposit<'info> {
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
    pub vault_spl: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: tmp no check
    pub owner_spl: UncheckedAccount<'info>,

    pub spl_mint: Box<Account<'info, Mint>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(
        mut,
        seeds = [b"user".as_ref(), vault.key().as_ref(), (0u16).to_le_bytes().as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_user: UncheckedAccount<'info>,
    
    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(
        mut,
        seeds = [b"user_stats".as_ref(), vault.key().as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_user_stats: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(
        mut,
        seeds = [b"drift_state".as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_state: UncheckedAccount<'info>,
    
    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    /// CHECK: Account is safe once address is correct
    #[account(address = instructions::ID)]
    instructions: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn auto_repay_deposit_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, AutoRepayDeposit<'info>>, 
    amount_base_units: u64,
    drift_market_index: u16
) -> Result<()> {
    let index = load_current_index_checked(&ctx.accounts.instructions.to_account_info())?;
    let withdraw_instruction = load_instruction_at_checked(index as usize + 1, &ctx.accounts.instructions.to_account_info())?;
    let swap_instruction = load_instruction_at_checked(index as usize + 2, &ctx.accounts.instructions.to_account_info())?;
    let check_instruction = load_instruction_at_checked(index as usize + 3, &ctx.accounts.instructions.to_account_info())?;

    msg!("index: {}", index);
    msg!("withdraw_instruction: {:?}", withdraw_instruction);
    msg!("swap_instruction: {:?}", swap_instruction);
    msg!("check_instruction: {:?}", check_instruction);

    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    // Transfer tokens from owner's ATA to vault's ATA

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            token::Transfer { 
                from: ctx.accounts.owner_spl.to_account_info(), 
                to: ctx.accounts.vault_spl.to_account_info(), 
                authority: ctx.accounts.owner.to_account_info()
            }
        ),
        amount_base_units
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
        signer_seeds
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    // reduce_only = true to prevent depositing more than the borrowed position
    drift_deposit(cpi_ctx, drift_market_index, amount_base_units, true)?;

    // Close vault's ATA

    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::CloseAccount {
            account: ctx.accounts.vault_spl.to_account_info(),
            destination: ctx.accounts.owner.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds
    );
    token::close_account(cpi_ctx_close)?;

    Ok(())
}