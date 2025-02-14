use crate::{config::INIT_ACCOUNT_RENT_FEE, state::Vault};
use crate::utils::validate_user_lookup_table;
use anchor_lang::prelude::*;
use solana_program::program::invoke_signed;
use drift::{
    program::Drift,
    cpi::{
        initialize_user as initialize_user_drift, 
        initialize_user_stats as initialize_user_stats_drift
    }, 
    cpi::accounts::InitializeUser as InitializeUserDrift, 
    cpi::accounts::InitializeUserStats as InitializeUserStatsDrift,
    state::state::State as DriftState
};
use solana_program::system_instruction;

#[derive(Accounts)]
pub struct InitUser<'info> {
    #[account(
        init,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump,
        payer = init_rent_payer,
        space = Vault::INIT_SPACE
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub owner: Signer<'info>,

    // CHECK: This account is checked by validate_user_lookup_table
    pub lookup_table: UncheckedAccount<'info>,

    /// CHECK: This account is safe once the seeds are correct
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

    #[account(
        mut,
        seeds = [b"drift_state".as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_state: Box<Account<'info, DriftState>>,

    pub drift_program: Program<'info, Drift>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,
}

pub fn init_user_handler(
    ctx: Context<InitUser>,
    requires_marginfi_account: bool,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    extend_spend_limit_per_timeframe_reset_slot_amount: u64
) -> Result<()> {
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    // Pay init_rent_payer the init fee (minus address lookup table rent, built client-side)
    let address_lookup_table_rent = 0; // TODO - calcualte required rent for table
    invoke_signed(
        &system_instruction::transfer(
            ctx.accounts.owner.key, 
            ctx.accounts.init_rent_payer.key, 
            INIT_ACCOUNT_RENT_FEE - address_lookup_table_rent
        ),
        &[
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.init_rent_payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    // Init vault
    ctx.accounts.vault.owner = ctx.accounts.owner.key();
    ctx.accounts.vault.bump = ctx.bumps.vault;

    validate_user_lookup_table(
        &ctx.accounts.lookup_table,
        &ctx.accounts.owner.key(),
        &ctx.remaining_accounts
    )?;
    ctx.accounts.vault.lookup_table = ctx.accounts.lookup_table.key();

    ctx.accounts.vault.spend_limit_per_transaction = spend_limit_per_transaction;
    ctx.accounts.vault.spend_limit_per_timeframe = spend_limit_per_timeframe;
    ctx.accounts.vault.remaining_spend_limit_per_timeframe = spend_limit_per_timeframe;
    ctx.accounts.vault.extend_spend_limit_per_timeframe_reset_slot_amount = extend_spend_limit_per_timeframe_reset_slot_amount;

    let current_slot = Clock::get()?.slot;
    ctx.accounts.vault.next_spend_limit_per_timeframe_reset_slot = current_slot + extend_spend_limit_per_timeframe_reset_slot_amount;

    // Init integrations
    init_drift_accounts(&ctx, signer_seeds)?;
    if requires_marginfi_account {
        init_marginfi_account(&ctx)?;
    }

    Ok(())
}

fn init_drift_accounts(
    ctx: &Context<InitUser>,
    signer_seeds: &[&[&[u8]]]
) -> Result<()> {
    let create_user_stats_cpi_context = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        InitializeUserStatsDrift {
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            state: ctx.accounts.drift_state.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            payer: ctx.accounts.init_rent_payer.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        },
        signer_seeds
    );
    initialize_user_stats_drift(create_user_stats_cpi_context)?;
    
    let create_user_cpi_context = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        InitializeUserDrift {
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            state: ctx.accounts.drift_state.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            payer: ctx.accounts.init_rent_payer.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        },
        signer_seeds
    );
    initialize_user_drift(create_user_cpi_context, 0, [0; 32])?;

    Ok(())
}

fn init_marginfi_account(
    ctx: &Context<InitUser>
) -> Result<()> {
    // TODO: Create marginfi account, paying with init_rent_payer

    Ok(())
}