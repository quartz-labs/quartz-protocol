use crate::config::QuartzError;
use crate::config::DRIFT_MARKETS;
use crate::config::{TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};
use crate::state::Vault;
use crate::utils::{
    get_ata_public_key, get_drift_user_public_key, get_drift_user_stats_public_key,
    get_vault_spl_public_key,
};
use anchor_lang::prelude::*;
use solana_address_lookup_table_program::{
    instruction::{create_lookup_table, extend_lookup_table},
    ID as ADDRESS_LOOKUP_TABLE_PROGRAM_ID,
};
use solana_address_lookup_table_program::state::AddressLookupTable;
use solana_program::program::invoke;

#[derive(Accounts)]
pub struct InitializeUser<'info> {
    #[account(
        init,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump,
        payer = owner,
        space = Vault::INIT_SPACE
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Address lookup table account, created by the instruction
    #[account(mut)]
    pub lookup_table: UncheckedAccount<'info>,

    /// CHECK: Account is safe once ID is correct
    #[account(
        constraint = address_lookup_table_program.key() == ADDRESS_LOOKUP_TABLE_PROGRAM_ID
    )]
    pub address_lookup_table_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn init_user_handler(
    ctx: Context<InitializeUser>,
    spend_balance_amount: u64,
    slot: u64,
) -> Result<()> {
    ctx.accounts.vault.owner = ctx.accounts.owner.key();
    ctx.accounts.vault.bump = ctx.bumps.vault;
    ctx.accounts.vault.spend_balance_amount = spend_balance_amount;

    //check that look up table is not empty and  initialized and that the owner is the user
    if ctx.accounts.lookup_table.data_len() == 0
        || ctx.accounts.lookup_table.lamports() == 0
        || ctx.accounts.lookup_table.owner != &ADDRESS_LOOKUP_TABLE_PROGRAM_ID
    {
        return err!(QuartzError::InvalidLookupTable);
    }

    //check that the first account is the vault
    //serialize the lookup table account as a Lookuptable type
    let lookup_table = AddressLookupTable::deserialize(&ctx.accounts.lookup_table.data.borrow()).unwrap();


    if lookup_table.meta.authority != Some(ctx.accounts.owner.key()) {
        return err!(QuartzError::InvalidLookupTableAuthority);
    }

    //get the accounts array from the lookup table
    let lut_accounts = lookup_table.addresses.to_vec();


    //check that the first account is the vault
    if lut_accounts[0] != ctx.accounts.vault.key() {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    //check that the second account is the owner
    if lut_accounts[1] != ctx.accounts.owner.key() {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    //check that the third is the drift_user
    let drift_user = get_drift_user_public_key(&ctx.accounts.vault.key());
    if lut_accounts[2] != drift_user {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    //check that the fourth is the drift_user_stats
    let drift_user_stats = get_drift_user_stats_public_key(&ctx.accounts.vault.key());
    if lut_accounts[3] != drift_user_stats {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    for lut_account in lut_accounts[4..].iter() {
//check that the account


    }

    //for the rest of the accounts, check that they are valid by 
    for market in DRIFT_MARKETS.iter() {
        let vault_spl = get_vault_spl_public_key(&ctx.accounts.owner.key(), &market.mint);
        accounts.push(vault_spl);

        if let Some(mint_account) = ctx
            .remaining_accounts
            .iter()
            .find(|acc| acc.key() == market.mint)
        {
            let token_program_id = mint_account.owner;

            let ata = if token_program_id == &TOKEN_PROGRAM_ID {
                get_ata_public_key(&ctx.accounts.owner.key(), &market.mint, &TOKEN_PROGRAM_ID)
            } else if token_program_id == &TOKEN_2022_PROGRAM_ID {
                get_ata_public_key(
                    &ctx.accounts.owner.key(),
                    &market.mint,
                    &TOKEN_2022_PROGRAM_ID,
                )
            } else {
                msg!("Invalid token program id during loop: {}", token_program_id);
                return err!(QuartzError::InvalidTokenProgramId);
            };

        } else {
            return err!(QuartzError::MissingTokenMint);
        }
    }
    Ok(())
}
