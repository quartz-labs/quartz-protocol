use crate::{
    check,
    config::{QuartzError, TIME_LOCK_RENT_PAYER_SEEDS},
    state::{TimeLock, TimeLocked},
};
use anchor_lang::prelude::*;
use solana_program::{
    program::{invoke, invoke_signed},
    rent::Rent,
    system_instruction,
};

pub fn allocate_time_lock_program_payer<'info>(
    time_lock_rent_payer: &AccountInfo<'info>,
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: usize,
) -> Result<()> {
    let (time_lock_rent_payer_seeds, bump) = validate_time_lock_rent_payer(time_lock_rent_payer)?;

    let seeds_with_bump = &[time_lock_rent_payer_seeds, &[bump]];
    let signer_seeds = &[&seeds_with_bump[..]];

    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(space);

    // Transfer required lamports
    invoke_signed(
        &system_instruction::transfer(
            &time_lock_rent_payer.key(),
            &time_lock.key(),
            required_lamports,
        ),
        &[
            time_lock_rent_payer.to_account_info(),
            time_lock.to_account_info(),
            system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    allocate_time_lock(time_lock, system_program, space)?;

    Ok(())
}

fn validate_time_lock_rent_payer<'info>(
    time_lock_rent_payer: &AccountInfo<'info>,
) -> Result<(&'info [u8], u8)> {
    let time_lock_rent_payer_seeds = TIME_LOCK_RENT_PAYER_SEEDS;
    let (expected_pda, bump) =
        Pubkey::find_program_address(&[time_lock_rent_payer_seeds], &crate::ID);

    check!(
        time_lock_rent_payer.key().eq(&expected_pda),
        QuartzError::InvalidTimeLockRentPayer
    );

    Ok((time_lock_rent_payer_seeds, bump))
}

pub fn allocate_time_lock_owner_payer<'info>(
    owner: &Signer<'info>,
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: usize,
) -> Result<()> {
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(space);

    invoke(
        &system_instruction::transfer(&owner.key(), &time_lock.key(), required_lamports),
        &[
            owner.to_account_info(),
            time_lock.to_account_info(),
            system_program.to_account_info(),
        ],
    )?;

    allocate_time_lock(time_lock, system_program, space)?;

    Ok(())
}

fn allocate_time_lock<'info>(
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: usize,
) -> Result<()> {
    // Allocate data
    invoke(
        &system_instruction::allocate(&time_lock.key(), space as u64),
        &[
            time_lock.to_account_info(),
            system_program.to_account_info(),
        ],
    )?;

    // Change ownership to program
    invoke(
        &system_instruction::assign(&time_lock.key(), &crate::ID),
        &[
            time_lock.to_account_info(),
            system_program.to_account_info(),
        ],
    )?;

    Ok(())
}

pub fn validate_time_lock(owner: &Pubkey, time_lock: &TimeLock) -> Result<()> {
    check!(time_lock.owner.eq(owner), QuartzError::InvalidTimeLockOwner);

    let current_slot = Clock::get()?.slot;
    check!(
        time_lock.release_slot <= current_slot,
        QuartzError::TimeLockNotReleased
    );

    Ok(())
}

pub fn close_time_lock<'info, T>(
    time_lock: &Account<'info, T>,
    time_lock_rent_payer: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
) -> Result<()>
where
    T: TimeLocked + AccountSerialize + AccountDeserialize + Clone,
{
    let destination = if time_lock.time_lock().is_owner_payer {
        owner
    } else {
        validate_time_lock_rent_payer(time_lock_rent_payer)?;
        time_lock_rent_payer
    };

    // Transfer all rent to payer
    let time_lock_balance = time_lock.to_account_info().lamports();
    **destination.lamports.borrow_mut() = destination
        .lamports()
        .checked_add(time_lock_balance)
        .ok_or(QuartzError::MathOverflow)?;
    **time_lock.to_account_info().lamports.borrow_mut() = 0;

    // Clear data
    time_lock.to_account_info().data.borrow_mut().fill(0);

    Ok(())
}
