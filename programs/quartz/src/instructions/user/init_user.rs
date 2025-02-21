use crate::{check, config::{QuartzError, ANCHOR_DISCRIMINATOR, INIT_ACCOUNT_RENT_FEE, MARGINFI_ACCOUNT_INITIALIZE_DISCRIMINATOR, MARGINFI_GROUP_1, MARGINFI_PROGRAM_ID}, state::Vault};
use anchor_lang::{prelude::*, system_program::{create_account, CreateAccount}, Discriminator};
use solana_program::program::{invoke, invoke_signed};
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

    #[account(
        mut,
        seeds = [b"drift_state".as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_state: Box<Account<'info, DriftState>>,

    pub drift_program: Program<'info, Drift>,

    /// CHECK: Safe once address is correct
    #[account(
        constraint = marginfi_group.key().eq(&MARGINFI_GROUP_1)
    )]
    pub marginfi_group: UncheckedAccount<'info>,

    #[account(mut)]
    pub marginfi_account: Signer<'info>,

    /// CHECK: Safe once address is correct
    #[account(
        constraint = marginfi_program.key().eq(&MARGINFI_PROGRAM_ID)
    )]
    pub marginfi_program: UncheckedAccount<'info>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,
}

pub fn init_user_handler(
    ctx: Context<InitUser>,
    requires_marginfi_account: bool,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64
) -> Result<()> {
    let vault_bump = ctx.bumps.vault;
    let owner = ctx.accounts.owner.key();
    let seeds_vault = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];

    let init_rent_payer_bump = ctx.bumps.init_rent_payer;
    let init_rent_payer_seeds = &[
        b"init_rent_payer".as_ref(),
        &[init_rent_payer_bump]
    ];

    let init_rent_payer_signer_seeds = &[
        &init_rent_payer_seeds[..]
    ];
    let both_signer_seeds = &[
        &init_rent_payer_seeds[..],
        &seeds_vault[..]
    ];

    // Check vault is not already initialized
    check!(
        ctx.accounts.vault.data_is_empty(),
        QuartzError::VaultAlreadyInitialized
    );

    // Pay init_rent_payer the init fee
    invoke(
        &system_instruction::transfer(
            ctx.accounts.owner.key, 
            ctx.accounts.init_rent_payer.key, 
            INIT_ACCOUNT_RENT_FEE
        ),
        &[
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.init_rent_payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    init_vault(
        &ctx, 
        both_signer_seeds, 
        spend_limit_per_transaction, 
        spend_limit_per_timeframe, 
        timeframe_in_seconds,
        next_timeframe_reset_timestamp
    )?;

    init_drift_accounts(&ctx, both_signer_seeds)?;

    if requires_marginfi_account {
        init_marginfi_account(&ctx, init_rent_payer_signer_seeds)?;
    }

    Ok(())
}

fn init_vault(
    ctx: &Context<InitUser>,
    both_signer_seeds: &[&[&[u8]]],
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64
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
            both_signer_seeds
        ),
        vault_rent_required,
        Vault::INIT_SPACE as u64,
        &crate::ID
    )?;

    // Init vault data
    let vault_data = Vault {
        owner: ctx.accounts.owner.key(),
        bump: ctx.bumps.vault,
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        remaining_spend_limit_per_timeframe: spend_limit_per_timeframe,
        next_timeframe_reset_timestamp,
        timeframe_in_seconds
    };
    let vault_data_vec = vault_data.try_to_vec()?;

    let mut new_account_data = ctx.accounts.vault.try_borrow_mut_data()?;
    new_account_data[..ANCHOR_DISCRIMINATOR].copy_from_slice(&Vault::DISCRIMINATOR);
    new_account_data[ANCHOR_DISCRIMINATOR..].copy_from_slice(&vault_data_vec[..]);

    Ok(())
}

fn init_drift_accounts(
    ctx: &Context<InitUser>,
    both_signer_seeds: &[&[&[u8]]]
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
        both_signer_seeds
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
        both_signer_seeds
    );
    initialize_user_drift(create_user_cpi_context, 0, [0; 32])?;

    Ok(())
}

fn init_marginfi_account(
    ctx: &Context<InitUser>,
    init_rent_payer_signer_seeds: &[&[&[u8]]]
) -> Result<()> {
    let ix = solana_program::instruction::Instruction {
        program_id: ctx.accounts.marginfi_program.key(),
        accounts: vec![
            AccountMeta::new_readonly(ctx.accounts.marginfi_group.key(), false),
            AccountMeta::new(ctx.accounts.marginfi_account.key(), true),
            AccountMeta::new(ctx.accounts.owner.key(), true),
            AccountMeta::new(ctx.accounts.init_rent_payer.key(), true),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ],
        data: MARGINFI_ACCOUNT_INITIALIZE_DISCRIMINATOR.to_vec(),
    };

    invoke_signed(
        &ix,
        &[
            ctx.accounts.marginfi_group.to_account_info(),
            ctx.accounts.marginfi_account.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.init_rent_payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        init_rent_payer_signer_seeds
    )?;

    Ok(())
}