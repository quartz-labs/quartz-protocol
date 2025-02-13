use crate::config::{QuartzError, ANCHOR_DISCRIMINATOR, PUBKEY_SIZE};
use crate::state::Vault;
use crate::utils::validate_user_lookup_table;
use anchor_lang::prelude::*;

use solana_program::address_lookup_table_account::AddressLookupTableAccount;
use solana_program::{program::invoke_signed, system_instruction};

#[derive(Accounts)]
pub struct UpgradeVault<'info> {
    /// CHECK: Account is checked by handler
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>, // TODO: If any weird issues, change back to AccountInfo

    #[account(mut)]
    pub owner: Signer<'info>,

    pub lookup_table: Box<Account<'info, AddressLookupTableAccount>>,

    pub system_program: Program<'info, System>,
}

pub fn upgrade_vault_handler(ctx: Context<UpgradeVault>) -> Result<()> {
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
    validate_user_lookup_table(lookup_table)?;

    // Get new vault data and required size
    let new_vault = Vault {
        owner: ctx.accounts.owner.key(),
        bump: vault_bump,
        lookup_table: ctx.accounts.lookup_table.key(),
        spend_balance_amount: 0,
        // TODO: Add in data for time block for spend_balance_amount
    };
    let new_vault_vec = new_vault.try_to_vec().unwrap();

    let rent = Rent::get()?;
    let new_minimum_balance = rent.minimum_balance(Vault::INIT_SPACE);
    let lamports_diff = new_minimum_balance
        .checked_sub(existing_vault.lamports())
        .ok_or(QuartzError::MathOverflow)?;

    // Extend the vault size
    let seeds = &[
        b"vault",
        ctx.accounts.owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    invoke_signed(
        &system_instruction::transfer(
            ctx.accounts.owner.key, 
            existing_vault.key, 
            lamports_diff
        ),
        &[
            ctx.accounts.owner.to_account_info(),
            existing_vault.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    // Reallocate data
    existing_vault.realloc(Vault::INIT_SPACE, false)?;
    let mut vault_data = existing_vault.try_borrow_mut_data()?;
    vault_data[ANCHOR_DISCRIMINATOR..].copy_from_slice(&new_vault_vec[ANCHOR_DISCRIMINATOR..]);

    Ok(())
}
