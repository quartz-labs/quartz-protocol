use crate::{
    check,
    config::{QuartzError, ANCHOR_DISCRIMINATOR, INIT_ACCOUNT_RENT_FEE},
    state::Vault,
};
use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
    Discriminator,
};
use drift::{
    cpi::accounts::InitializeUser as InitializeUserDrift,
    cpi::accounts::InitializeUserStats as InitializeUserStatsDrift,
    cpi::{
        initialize_user as initialize_user_drift,
        initialize_user_stats as initialize_user_stats_drift,
    },
    program::Drift,
};
use solana_program::system_instruction;
use solana_program::{program::invoke, system_program};

#[derive(Accounts)]
pub struct InitUser<'info> {
    /// CHECK: Safe once address is correct
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

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

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,
}

pub fn init_user_handler(
    ctx: Context<InitUser>,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64,
) -> Result<()> {
    let vault_bump = ctx.bumps.vault;
    let owner = ctx.accounts.owner.key();
    let seeds_vault = &[b"vault", owner.as_ref(), &[vault_bump]];

    let init_rent_payer_bump = ctx.bumps.init_rent_payer;
    let init_rent_payer_seeds = &[b"init_rent_payer".as_ref(), &[init_rent_payer_bump]];
    let signer_seeds = &[&init_rent_payer_seeds[..], &seeds_vault[..]];

    // Check vault is not already initialized
    check!(
        ctx.accounts.vault.owner.key().eq(&system_program::ID),
        QuartzError::VaultAlreadyInitialized
    );
    check!(
        ctx.accounts.vault.lamports() == 0,
        QuartzError::VaultAlreadyInitialized
    );
    check!(
        ctx.accounts.vault.data_is_empty(),
        QuartzError::VaultAlreadyInitialized
    );

    // Pay init_rent_payer the init fee
    invoke(
        &system_instruction::transfer(
            ctx.accounts.owner.key,
            ctx.accounts.init_rent_payer.key,
            INIT_ACCOUNT_RENT_FEE,
        ),
        &[
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.init_rent_payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    init_vault(
        &ctx,
        signer_seeds,
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        timeframe_in_seconds,
        next_timeframe_reset_timestamp,
    )?;

    init_drift_accounts(&ctx, signer_seeds)?;

    Ok(())
}

fn init_vault(
    ctx: &Context<InitUser>,
    signer_seeds: &[&[&[u8]]],
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64,
) -> Result<()> {
    // Init vault space
    let rent = Rent::get()?;
    let vault_rent_required = rent.minimum_balance(Vault::INIT_SPACE);
    create_account(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            CreateAccount {
                from: ctx.accounts.init_rent_payer.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
            signer_seeds,
        ),
        vault_rent_required,
        Vault::INIT_SPACE as u64,
        &crate::ID,
    )?;

    // Init vault data
    let vault_data = Vault {
        owner: ctx.accounts.owner.key(),
        bump: ctx.bumps.vault,
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        remaining_spend_limit_per_timeframe: spend_limit_per_timeframe,
        next_timeframe_reset_timestamp,
        timeframe_in_seconds,
    };
    let vault_data_vec = vault_data.try_to_vec()?;

    let mut new_account_data = ctx.accounts.vault.try_borrow_mut_data()?;
    new_account_data[..ANCHOR_DISCRIMINATOR].copy_from_slice(&Vault::DISCRIMINATOR);
    new_account_data[ANCHOR_DISCRIMINATOR..].copy_from_slice(&vault_data_vec[..]);

    Ok(())
}

fn init_drift_accounts(ctx: &Context<InitUser>, both_signer_seeds: &[&[&[u8]]]) -> Result<()> {
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
        both_signer_seeds,
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
        both_signer_seeds,
    );
    initialize_user_drift(create_user_cpi_context, 0, [0; 32])?;

    Ok(())
}
