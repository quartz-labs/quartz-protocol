use anchor_lang::prelude::*;
use crate::config::DRIFT_ID;

pub fn get_drift_user_public_key(vault_pda: &Pubkey) -> Pubkey {
    let seeds = [
        b"user",
        vault_pda.as_ref(),
        &0u16.to_le_bytes(),
    ];
    
    let (user_pda, _bump) = Pubkey::find_program_address(
        &seeds,
        &DRIFT_ID
    );
    
    user_pda
}

pub fn get_drift_user_stats_public_key(vault_pda: &Pubkey) -> Pubkey {
    let seeds = [
        b"user_stats",
        vault_pda.as_ref(),
    ];
    
    let (user_stats_pda, _bump) = Pubkey::find_program_address(
        &seeds,
        &DRIFT_ID
    );
    
    user_stats_pda
}

pub fn get_vault_public_key(user: &Pubkey) -> Pubkey {
    let seeds = [
        b"vault",
        user.as_ref(),
    ];
    
    let (vault_pda, _bump) = Pubkey::find_program_address(
        &seeds,
        &crate::id()
    );
    
    vault_pda
}

pub fn get_vault_spl_public_key(user: &Pubkey, mint: &Pubkey) -> Pubkey {
    let vault_pda = get_vault_public_key(user);
    let seeds = [
        vault_pda.as_ref(),
        mint.as_ref(),
    ];
    
    let (vault_spl_pda, _bump) = Pubkey::find_program_address(
        &seeds,
        &crate::id()
    );
    
    vault_spl_pda
}

pub fn get_ata_public_key(user: &Pubkey, mint: &Pubkey, token_program_id: &Pubkey) -> Pubkey {
    let seeds = [
        user.as_ref(),
        mint.as_ref(),
    ];

    let (ata_pda, _bump) = Pubkey::find_program_address(
        &seeds,
        &token_program_id
    );

    ata_pda
}