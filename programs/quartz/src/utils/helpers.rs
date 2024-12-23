use anchor_lang::prelude::*;
use solana_program::instruction::Instruction;
use crate::{
    check, config::{QuartzError, DRIFT_MARKETS, U16_SIZE, U64_SIZE, U8_SIZE}, state::DriftMarket
};

pub fn get_jup_exact_out_route_platform_fees(instruction: &Instruction) -> Result<u8> {
    let platform_fee_index_start = instruction.data.len() - U8_SIZE;

    let platform_fee_bps = instruction.data[platform_fee_index_start..]
        .try_into()
        .map_err(|_| QuartzError::DeserializationError)?;

    Ok(u8::from_le_bytes(platform_fee_bps))
}

pub fn get_jup_exact_out_route_out_amount(instruction: &Instruction) -> Result<u64> {
    let out_amount_index_start = instruction.data.len() - (U8_SIZE + U16_SIZE + U64_SIZE + U64_SIZE);
    let out_amount_index_end = out_amount_index_start + U64_SIZE;

    let out_amount = instruction.data[out_amount_index_start..out_amount_index_end]
        .try_into()
        .map_err(|_| QuartzError::DeserializationError)?;

    Ok(u64::from_le_bytes(out_amount))
}

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