use crate::{
    check,
    config::{
        DriftMarket, QuartzError, ANCHOR_DISCRIMINATOR, DRIFT_MARKETS, TIME_LOCK_RENT_PAYER_SEEDS,
    },
    state::{TimeLock, TimeLocked},
};
use anchor_lang::{prelude::*, Discriminator};
use solana_program::{
    instruction::{get_stack_height, Instruction},
    program::{invoke, invoke_signed},
    system_instruction, system_program,
};

pub fn get_drift_market(market_index: u16) -> Result<&'static DriftMarket> {
    Ok(DRIFT_MARKETS
        .iter()
        .find(|market| market.market_index == market_index)
        .ok_or(QuartzError::InvalidMarketIndex)?)
}

pub fn normalize_price_exponents(
    price_a: u128,
    exponent_a: i32,
    price_b: u128,
    exponent_b: i32,
) -> Result<(u128, u128)> {
    // Used to compare two oracle prices
    let exponent_difference = exponent_a
        .checked_sub(exponent_b)
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
        return Ok((price_a, price_b));
    }

    if exponent_difference > 0 {
        // a > b
        let amount_b_normalized = (price_b)
            .checked_mul(10_u128.pow(exponent_difference.unsigned_abs()))
            .ok_or(QuartzError::MathOverflow)?;
        Ok((price_a, amount_b_normalized))
    } else {
        // b > a
        let amount_a_normalized = (price_a)
            .checked_mul(10_u128.pow(exponent_difference.unsigned_abs()))
            .ok_or(QuartzError::MathOverflow)?;
        Ok((amount_a_normalized, price_b))
    }
}

pub fn validate_start_collateral_repay_ix(
    current_instruction: &Instruction,
    start_collateral_repay: &Instruction,
) -> Result<()> {
    // Ensure we're not in a CPI
    const TOP_LEVEL_STACK_HEIGHT: usize = 1;
    check!(
        get_stack_height() == TOP_LEVEL_STACK_HEIGHT,
        QuartzError::IllegalCollateralRepayCPI
    );
    check!(
        current_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayCPI
    );

    // Ensure start instruction is valid
    check!(
        start_collateral_repay.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        start_collateral_repay.data[..ANCHOR_DISCRIMINATOR]
            .eq(&crate::instruction::StartCollateralRepay::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    Ok(())
}

pub fn evm_address_to_solana(ethereum_address: &str) -> Result<Pubkey> {
    // Used for Circle bridge
    let cleaned_address = ethereum_address.trim_start_matches("0x");
    check!(cleaned_address.len() == 40, QuartzError::InvalidEvmAddress);

    let mut bytes = [0u8; 32];
    for i in 0..20 {
        let pos = i * 2;
        let byte_str = &cleaned_address[pos..pos + 2];
        bytes[i + 12] =
            u8::from_str_radix(byte_str, 16).map_err(|_| QuartzError::InvalidEvmAddress)?;
    }

    Ok(Pubkey::new_from_array(bytes))
}

fn validate_time_lock_fresh(time_lock: &AccountInfo) -> Result<()> {
    check!(
        time_lock.owner.key().eq(&system_program::ID),
        QuartzError::TimeLockAlreadyInitialized
    );
    check!(
        time_lock.lamports() == 0,
        QuartzError::TimeLockAlreadyInitialized
    );
    check!(
        time_lock.data_is_empty(),
        QuartzError::TimeLockAlreadyInitialized
    );

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

pub fn allocate_time_lock_program_payer<'info>(
    time_lock_rent_payer: &AccountInfo<'info>,
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: usize,
) -> Result<()> {
    validate_time_lock_fresh(time_lock)?;

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

    allocate_time_lock(
        time_lock,
        system_program,
        u64::try_from(space).map_err(|_| QuartzError::MathOverflow)?,
    )?;

    Ok(())
}

pub fn allocate_time_lock_owner_payer<'info>(
    owner: &Signer<'info>,
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: usize,
) -> Result<()> {
    validate_time_lock_fresh(time_lock)?;

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

    allocate_time_lock(
        time_lock,
        system_program,
        u64::try_from(space).map_err(|_| QuartzError::MathOverflow)?,
    )?;

    Ok(())
}

fn allocate_time_lock<'info>(
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: u64,
) -> Result<()> {
    // Allocate data
    invoke(
        &system_instruction::allocate(&time_lock.key(), space),
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
