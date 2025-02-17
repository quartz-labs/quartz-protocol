use anchor_lang::prelude::*;
use drift::{
    instructions::optional_accounts::{load_maps, AccountMaps}, 
    math::margin::{calculate_margin_requirement_and_total_collateral_and_liability_info, MarginRequirementType}, 
    state::{
        margin_calculation::{MarginCalculation, MarginContext}, 
        spot_market_map::get_writable_spot_market_set_from_many, state::State, user::User
    }  
};
use std::collections::BTreeSet;

use crate::config::{QuartzError, ACCOUNT_HEALTH_BUFFER_PERCENT};

pub(crate) type MarketSet = BTreeSet<u16>;

pub fn calculate_initial_margin_requirement<'info>(
    drift_user: &User,
    drift_state: &State,
    market_index_asset: u16,
    market_index_liability: u16,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<MarginCalculation> {
    let clock = Clock::get()?;
    let remaining_accounts_iter = &mut remaining_accounts.iter().peekable();
    
    let AccountMaps {
        perp_market_map,
        spot_market_map,
        mut oracle_map,
    } = load_maps(
        remaining_accounts_iter,
        &MarketSet::new(),
        &get_writable_spot_market_set_from_many(vec![market_index_asset, market_index_liability]),
        clock.slot,
        Some(drift_state.oracle_guard_rails),
    )?;

    let margin_context = MarginContext::standard(MarginRequirementType::Initial)
        .strict(true);

    let margin_calculation = calculate_margin_requirement_and_total_collateral_and_liability_info(
        drift_user,
        &perp_market_map,
        &spot_market_map,
        &mut oracle_map,
        margin_context
    )?;

    Ok(margin_calculation)
}

pub fn check_can_auto_repay(
    margin_calculation: MarginCalculation,
) -> Result<bool> {
    let has_sufficient_margin = margin_calculation.meets_margin_requirement();
    Ok(!has_sufficient_margin)
}

pub fn get_quartz_account_health(
    margin_calculation: MarginCalculation,
) -> Result<u8> {
    let total_collateral = margin_calculation.total_collateral;
    let margin_requirement = margin_calculation.margin_requirement;

    if total_collateral <= 0 || ACCOUNT_HEALTH_BUFFER_PERCENT >= 100 {
        return Ok(0);
    }

    if margin_requirement == 0 {
        return Ok(100);
    }

    let total_collateral_unsigned = total_collateral as u128;

    let buffer_multiplier = 100u128.checked_sub(ACCOUNT_HEALTH_BUFFER_PERCENT as u128)
        .ok_or(QuartzError::MathOverflow)?;
    
    let adjusted_total_collateral = total_collateral_unsigned
        .checked_mul(buffer_multiplier)
        .ok_or(QuartzError::MathOverflow)?
        .checked_div(100)
        .ok_or(QuartzError::MathOverflow)?;

    if margin_requirement > adjusted_total_collateral {
        return Ok(0);
    }

    let health = adjusted_total_collateral
        .checked_sub(margin_requirement)
        .ok_or(QuartzError::MathOverflow)?
        .checked_mul(100)
        .ok_or(QuartzError::MathOverflow)?
        .checked_div(adjusted_total_collateral)
        .ok_or(QuartzError::MathOverflow)?;

    Ok(health as u8)
}

fn _get_drift_account_health<'info>(
    margin_calculation: MarginCalculation,
) -> Result<u8> {
    let total_collateral = margin_calculation.total_collateral;
    let margin_requirement = margin_calculation.margin_requirement;

    if total_collateral < 0 {
        return Ok(0);
    }

    let total_collateral_unsigned = total_collateral as u128;

    if margin_requirement > total_collateral_unsigned {
        return Ok(0);
    }

    if margin_requirement == 0 {
        return Ok(100);
    }

    let health = total_collateral_unsigned.checked_sub(margin_requirement)
        .ok_or(QuartzError::MathOverflow)?
        .checked_mul(100)
        .ok_or(QuartzError::MathOverflow)?
        .checked_div(total_collateral_unsigned)
        .ok_or(QuartzError::MathOverflow)?;

    Ok(health as u8)
}
