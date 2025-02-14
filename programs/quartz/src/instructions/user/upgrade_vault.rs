use crate::config::{QuartzError, ANCHOR_DISCRIMINATOR, PUBKEY_SIZE};
use crate::state::Vault;
use crate::utils::validate_user_lookup_table;
use anchor_lang::prelude::*;
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

    // TODO: This account is checked by validate_user_lookup_table
    pub lookup_table: UncheckedAccount<'info>,

    /// CHECK: This account is safe once the seeds are correct
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
    extend_spend_limit_per_timeframe_reset_slot_amount: u64
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

    validate_user_lookup_table(
        &ctx.accounts.lookup_table, 
        &ctx.accounts.owner.key(),
        &ctx.remaining_accounts
    )?;

    // Get new vault data and required size
    let current_slot = Clock::get()?.slot;
    let new_vault = Vault {
        owner: ctx.accounts.owner.key(),
        bump: vault_bump,
        lookup_table: ctx.accounts.lookup_table.key(),
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        remaining_spend_limit_per_timeframe: spend_limit_per_timeframe,
        next_spend_limit_per_timeframe_reset_slot: current_slot + extend_spend_limit_per_timeframe_reset_slot_amount,
        extend_spend_limit_per_timeframe_reset_slot_amount
    };
    let new_vault_vec = new_vault.try_to_vec().unwrap();

    let rent = Rent::get()?;
    let new_minimum_balance = rent.minimum_balance(Vault::INIT_SPACE);
    let lamports_diff = new_minimum_balance
        .checked_sub(existing_vault.lamports())
        .ok_or(QuartzError::MathOverflow)?;

    // Extend the vault size
    let owner_key = ctx.accounts.owner.key();
    let seeds = &[
        b"init_rent_payer",
        &[vault_bump]
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
    vault_data[ANCHOR_DISCRIMINATOR..].copy_from_slice(&new_vault_vec[ANCHOR_DISCRIMINATOR..]);

    Ok(())
}
