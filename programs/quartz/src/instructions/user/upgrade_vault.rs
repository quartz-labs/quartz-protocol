use crate::config::{QuartzError, ANCHOR_DISCRIMINATOR, PUBKEY_SIZE};
use crate::state::Vault;
use anchor_lang::prelude::*;
use solana_program::{program::invoke_signed, system_instruction};

#[derive(Accounts)]
pub struct UpgradeVault<'info> {
    /// CHECK: Safe once address is correct
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump
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

    pub system_program: Program<'info, System>,
}

pub fn upgrade_vault_handler(
    ctx: Context<UpgradeVault>,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_slots: u64
) -> Result<()> {
    // Get current Vault data
    let existing_vault = &ctx.accounts.vault;
    let (vault_owner, vault_bump) = {
        let bump_start_bytes = ANCHOR_DISCRIMINATOR + PUBKEY_SIZE;

        let data = existing_vault.data.borrow();
        let owner_bytes = &data[
            ANCHOR_DISCRIMINATOR
            ..
            bump_start_bytes
        ];
        let owner = Pubkey::new_from_array(owner_bytes.try_into().unwrap());

        (owner, data[bump_start_bytes])
    };

    // Validate accounts
    require_keys_eq!(
        vault_owner,
        ctx.accounts.owner.key(),
        QuartzError::InvalidVaultOwner
    );

    // Get new vault data and required size
    let current_slot = Clock::get()?.slot;
    let new_vault = Vault {
        owner: ctx.accounts.owner.key(),
        bump: vault_bump,
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        remaining_spend_limit_per_timeframe: spend_limit_per_timeframe,
        next_timeframe_reset_slot: current_slot + timeframe_in_slots,
        timeframe_in_slots
    };
    let new_vault_vec = new_vault.try_to_vec().unwrap();

    let rent = Rent::get()?;
    let new_minimum_balance = rent.minimum_balance(Vault::INIT_SPACE);
    let lamports_diff = new_minimum_balance
        .checked_sub(existing_vault.lamports())
        .ok_or(QuartzError::MathOverflow)?;

    // Extend the vault size
    let init_rent_payer_bump = ctx.bumps.init_rent_payer;
    let seeds = &[
        b"init_rent_payer".as_ref(),
        &[init_rent_payer_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    invoke_signed(
        &system_instruction::transfer(
            ctx.accounts.init_rent_payer.key, 
            existing_vault.key, 
            lamports_diff
        ),
        &[
            ctx.accounts.init_rent_payer.to_account_info(),
            existing_vault.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    // Reallocate data
    existing_vault.realloc(Vault::INIT_SPACE, false)?;
    let mut vault_data = existing_vault.try_borrow_mut_data()?;
    vault_data[ANCHOR_DISCRIMINATOR..].copy_from_slice(&new_vault_vec[..]);

    Ok(())
}
