use crate::{
    check,
    config::{
        DriftMarket, PyraError, ANCHOR_DISCRIMINATOR, DRIFT_MARKETS, TIME_LOCK_RENT_PAYER_SEEDS,
    },
    state::{TimeLock, TimeLocked},
};
use anchor_lang::{prelude::*, Discriminator};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{TokenAccount, TokenInterface},
};
use solana_program::{
    instruction::{get_stack_height, Instruction},
    program::{invoke, invoke_signed},
    system_instruction, system_program,
};

pub fn get_drift_market(market_index: u16) -> Result<&'static DriftMarket> {
    Ok(DRIFT_MARKETS
        .iter()
        .find(|market| market.market_index == market_index)
        .ok_or(PyraError::InvalidMarketIndex)?)
}

pub fn validate_account_fresh(account: &AccountInfo) -> Result<()> {
    check!(
        account.owner.key().eq(&system_program::ID),
        PyraError::AccountAlreadyInitialized
    );
    check!(
        account.lamports() == 0,
        PyraError::AccountAlreadyInitialized
    );
    check!(
        account.data_is_empty(),
        PyraError::AccountAlreadyInitialized
    );

    Ok(())
}

/// Normalizes two oracles prices (which are given in the form of price * 10^exponent) to be the same exponent
pub fn normalize_price_exponents(
    price_a: u128,
    exponent_a: i32,
    price_b: u128,
    exponent_b: i32,
) -> Result<(u128, u128)> {
    let exponent_difference = exponent_a
        .checked_sub(exponent_b)
        .ok_or(PyraError::MathOverflow)?;
    check!(
        exponent_difference != i32::MIN,
        PyraError::InvalidPriceExponent
    );

    // Sanity check on exponent difference
    check!(
        exponent_difference.unsigned_abs() <= 12,
        PyraError::InvalidPriceExponent
    );

    if exponent_difference == 0 {
        return Ok((price_a, price_b));
    }

    if exponent_difference > 0 {
        // exp(a) > exp(b), increase base value of a to match b's exponent
        let amount_a_normalized = (price_a)
            .checked_mul(10_u128.pow(exponent_difference.unsigned_abs()))
            .ok_or(PyraError::MathOverflow)?;
        Ok((amount_a_normalized, price_b))
    } else {
        // exp(b) > exp(a), increase base value of b to match a's exponent
        let amount_b_normalized = (price_b)
            .checked_mul(10_u128.pow(exponent_difference.unsigned_abs()))
            .ok_or(PyraError::MathOverflow)?;
        Ok((price_a, amount_b_normalized))
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
        PyraError::IllegalCollateralRepayCPI
    );
    check!(
        current_instruction.program_id.eq(&crate::id()),
        PyraError::IllegalCollateralRepayCPI
    );

    // Ensure start instruction is valid
    check!(
        start_collateral_repay.program_id.eq(&crate::id()),
        PyraError::IllegalCollateralRepayInstructions
    );

    check!(
        start_collateral_repay.data[..ANCHOR_DISCRIMINATOR]
            .eq(&crate::instruction::StartCollateralRepay::DISCRIMINATOR),
        PyraError::IllegalCollateralRepayInstructions
    );

    Ok(())
}

pub fn evm_address_to_solana(ethereum_address: &str) -> Result<Pubkey> {
    // Used for Circle bridge
    let cleaned_address = ethereum_address.trim_start_matches("0x");
    check!(cleaned_address.len() == 40, PyraError::InvalidEvmAddress);

    let mut bytes = [0u8; 32];
    for i in 0..20 {
        let pos = i * 2;
        let byte_str = &cleaned_address[pos..pos + 2];
        bytes[i + 12] =
            u8::from_str_radix(byte_str, 16).map_err(|_| PyraError::InvalidEvmAddress)?;
    }

    Ok(Pubkey::new_from_array(bytes))
}

fn validate_time_lock_rent_payer<'info>(
    time_lock_rent_payer: &AccountInfo<'info>,
) -> Result<(&'info [u8], u8)> {
    let time_lock_rent_payer_seeds = TIME_LOCK_RENT_PAYER_SEEDS;
    let (expected_pda, bump) =
        Pubkey::find_program_address(&[time_lock_rent_payer_seeds], &crate::ID);

    check!(
        time_lock_rent_payer.key().eq(&expected_pda),
        PyraError::InvalidTimeLockRentPayer
    );

    Ok((time_lock_rent_payer_seeds, bump))
}

pub fn allocate_time_lock_program_payer<'info>(
    time_lock_rent_payer: &AccountInfo<'info>,
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: usize,
) -> Result<()> {
    validate_account_fresh(time_lock)?;

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
        u64::try_from(space).map_err(|_| PyraError::MathOverflow)?,
    )?;

    Ok(())
}

pub fn allocate_time_lock_owner_payer<'info>(
    owner: &Signer<'info>,
    time_lock: &Signer<'info>,
    system_program: &Program<'info, System>,
    space: usize,
) -> Result<()> {
    validate_account_fresh(time_lock)?;

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
        u64::try_from(space).map_err(|_| PyraError::MathOverflow)?,
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
    check!(time_lock.owner.eq(owner), PyraError::InvalidTimeLockOwner);

    let current_slot = Clock::get()?.slot;
    check!(
        time_lock.release_slot < current_slot,
        PyraError::TimeLockNotReleased
    );

    Ok(())
}

pub fn close_time_lock<'info, T>(
    time_lock: &Account<'info, T>,
    time_lock_rent_payer: &AccountInfo<'info>,
) -> Result<()>
where
    T: TimeLocked + AccountSerialize + AccountDeserialize + Clone,
{
    if time_lock.time_lock().is_owner_payer {
        check!(
            time_lock_rent_payer.key().eq(&time_lock.time_lock().owner),
            PyraError::InvalidTimeLockRentPayer
        );
    } else {
        validate_time_lock_rent_payer(time_lock_rent_payer)?;
    };

    // Transfer all rent to payer
    let time_lock_balance = time_lock.to_account_info().lamports();
    **time_lock_rent_payer.lamports.borrow_mut() = time_lock_rent_payer
        .lamports()
        .checked_add(time_lock_balance)
        .ok_or(PyraError::MathOverflow)?;
    **time_lock.to_account_info().lamports.borrow_mut() = 0;

    // Clear data and owner
    time_lock.to_account_info().data.borrow_mut().fill(0);
    time_lock.to_account_info().owner = &system_program::ID;

    Ok(())
}

/// Validates associated token account. Returns Ok(Token Account) if it exists, Ok(None) if it doesn't exist, and Err if the seeds are invalid
pub fn validate_ata<'info>(
    ata: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    token_program: &Interface<'info, TokenInterface>,
) -> Result<Option<TokenAccount>> {
    // Validate seeds
    let expected_address = get_associated_token_address_with_program_id(
        &authority.key(),
        &mint.key(),
        &token_program.key(),
    );

    check!(
        ata.key().eq(&expected_address),
        PyraError::InvalidDepositAddressUSDC
    );

    // Check if exists
    if !ata.owner.eq(&token_program.key()) {
        return Ok(None);
    }

    if ata.data_is_empty() {
        return Ok(None);
    }

    let rent = Rent::get()?;
    if !rent.is_exempt(ata.lamports(), ata.data_len()) {
        return Ok(None);
    }

    let account = TokenAccount::try_deserialize(&mut &ata.data.borrow()[..])?;
    Ok(Some(account))
}
