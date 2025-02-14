use anchor_lang::{prelude::*, Discriminator};
use solana_address_lookup_table_program::state::AddressLookupTable;
use solana_program::instruction::Instruction;
use crate::{
    check, config::{QuartzError, ADDRESS_LOOKUP_TABLE_PROGRAM_ID, DRIFT_MARKETS, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID}, state::DriftMarket
};

use super::{get_ata_public_key, get_drift_user_public_key, get_drift_user_stats_public_key, get_vault_public_key, get_vault_spl_public_key};

pub fn get_drift_market(market_index: u16) -> Result<&'static DriftMarket> {
    Ok(DRIFT_MARKETS.iter().find(|market| market.market_index == market_index)
        .ok_or(QuartzError::InvalidMarketIndex)?)
}

pub fn normalize_price_exponents(price_a: u64, exponent_a: i32, price_b: u64, exponent_b: i32) -> Result<(u128, u128)> {
    let exponent_difference = exponent_a.checked_sub(exponent_b)
        .ok_or(QuartzError::MathOverflow)?;
    check!(
        exponent_difference != i32::MIN,
        QuartzError::InvalidPriceExponent
    );
    check!(
        exponent_difference.unsigned_abs() <= 32, // Sanity check on Pyth exponent difference
        QuartzError::InvalidPriceExponent
    );

    if exponent_difference == 0 {
        return Ok((price_a as u128, price_b as u128));
    }

    if exponent_difference > 0 {
        let amount_b_normalized = (price_b as u128)
            .checked_mul(10_u128.pow(exponent_difference.unsigned_abs() as u32))
            .ok_or(QuartzError::MathOverflow)?;
        return Ok((price_a as u128, amount_b_normalized));
    } else {
        let amount_a_normalized = (price_a as u128)
            .checked_mul(10_u128.pow(exponent_difference.unsigned_abs() as u32))
            .ok_or(QuartzError::MathOverflow)?;
        return Ok((amount_a_normalized, price_b as u128));
    }
}

pub fn validate_start_collateral_repay_ix(start_collateral_repay: &Instruction) -> Result<()> {
    check!(
        start_collateral_repay.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        start_collateral_repay.data[..8]
            .eq(&crate::instruction::StartCollateralRepay::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    Ok(())
}

pub fn evm_address_to_solana(ethereum_address: &str) -> Result<Pubkey> {
    let cleaned_address = ethereum_address.trim_start_matches("0x");
    check!(
        cleaned_address.len() == 40,
        QuartzError::InvalidEvmAddress
    );

    let mut bytes = [0u8; 32];
    for i in 0..20 {
        let pos = i * 2;
        let byte_str = &cleaned_address[pos..pos + 2];
        bytes[i + 12] = u8::from_str_radix(byte_str, 16)
            .map_err(|_| QuartzError::InvalidEvmAddress)?;
    }

    Ok(Pubkey::new_from_array(bytes))
}

pub fn validate_user_lookup_table(
    lookup_table: &UncheckedAccount,
    owner: &Pubkey,
    remaining_accounts: &[AccountInfo]
) -> Result<()> {
    // Check that lookup table is a valid address lookup table
    if lookup_table.data_len() == 0
        || lookup_table.lamports() == 0
        || lookup_table.owner != &ADDRESS_LOOKUP_TABLE_PROGRAM_ID
    {
        return err!(QuartzError::InvalidLookupTable);
    }
    
    // Get data and check that authority is the user
    let serialized_data = lookup_table.data.borrow();
    let lookup_table_data_result = AddressLookupTable::deserialize(&serialized_data);
    if lookup_table_data_result.is_err() {
        return err!(QuartzError::InvalidLookupTable);
    }

    let lookup_table_data = lookup_table_data_result.unwrap();

    if lookup_table_data.meta.authority != Some(*owner) {
        return err!(QuartzError::InvalidLookupTableAuthority);
    }

    // Check through default accounts
    let addresses = lookup_table_data.addresses.to_vec();

    if addresses.iter().find(|address| **address == *owner).is_none() {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    let vault = get_vault_public_key(owner);
    if addresses.iter().find(|address| **address == vault).is_none() {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    let drift_user = get_drift_user_public_key(&vault);
    if addresses.iter().find(|address| **address == drift_user).is_none() {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    let drift_user_stats = get_drift_user_stats_public_key(&vault);
    if addresses.iter().find(|address| **address == drift_user_stats).is_none() {
        return err!(QuartzError::InvalidLookupTableContent);
    }

    // Check all token dependant accounts 
    for market in DRIFT_MARKETS.iter() {
        // Get mint from remaining accounts
        let mint_account = remaining_accounts
            .iter()
            .find(|remaining_account| remaining_account.key() == market.mint);
    
        if mint_account.is_none() {
            return err!(QuartzError::MissingTokenMint);
        }
    
        let token_program_id = mint_account
            .expect("mint_account has passed existance check")
            .owner;

        if token_program_id != &TOKEN_PROGRAM_ID && token_program_id != &TOKEN_2022_PROGRAM_ID {
            return err!(QuartzError::InvalidTokenProgramId);
        }

        // Check ATA and Vault SPL are present
        let owner_ata = get_ata_public_key(owner, &market.mint, token_program_id);
        if addresses.iter().find(|address| **address == owner_ata).is_none() {
            return err!(QuartzError::InvalidLookupTableContent);
        }

        let vault_spl = get_vault_spl_public_key(owner, &market.mint);
        if addresses.iter().find(|address| **address == vault_spl).is_none() {
            return err!(QuartzError::InvalidLookupTableContent);
        }
    }

    Ok(())
}
