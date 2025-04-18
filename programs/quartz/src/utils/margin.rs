use crate::config::QuartzError;
use anchor_lang::prelude::*;
use drift::{
    instructions::optional_accounts::{load_maps, AccountMaps},
    math::margin::{
        calculate_margin_requirement_and_total_collateral_and_liability_info, MarginRequirementType,
    },
    state::{
        margin_calculation::{MarginCalculation, MarginContext},
        spot_market_map::get_writable_spot_market_set_from_many,
        state::State,
        user::User,
    },
};
use std::collections::BTreeSet;

pub(crate) type MarketSet = BTreeSet<u16>;

pub fn get_account_health<'info>(
    drift_user: &User,
    drift_state: &State,
    market_index_asset: u16,
    market_index_liability: u16,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<u8> {
    // Quartz health is calculated from initial margin (unlike Drift, which uses maintenance margin)
    let initial_margin_calculation = calculate_initial_margin_requirement(
        drift_user,
        drift_state,
        market_index_asset,
        market_index_liability,
        remaining_accounts,
    )?;

    calculate_quartz_account_health(initial_margin_calculation)
}

fn calculate_initial_margin_requirement<'info>(
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

    let margin_context = MarginContext::standard(MarginRequirementType::Initial);

    let margin_calculation = calculate_margin_requirement_and_total_collateral_and_liability_info(
        drift_user,
        &perp_market_map,
        &spot_market_map,
        &mut oracle_map,
        margin_context,
    )?;

    Ok(margin_calculation)
}

fn calculate_quartz_account_health(initial_margin_calculation: MarginCalculation) -> Result<u8> {
    let total_collateral = initial_margin_calculation.total_collateral;
    let margin_requirement = initial_margin_calculation.margin_requirement;

    if total_collateral < 0 {
        return Ok(0);
    }

    let total_collateral_unsigned =
        u128::try_from(total_collateral).map_err(|_| QuartzError::MathOverflow)?;

    if margin_requirement > total_collateral_unsigned {
        return Ok(0);
    }

    if margin_requirement == 0 {
        return Ok(100);
    }

    let health = total_collateral_unsigned
        .checked_sub(margin_requirement)
        .ok_or(QuartzError::MathOverflow)?
        .checked_mul(100)
        .ok_or(QuartzError::MathOverflow)?
        .checked_div(total_collateral_unsigned)
        .ok_or(QuartzError::MathOverflow)?;

    let health_u8 = u8::try_from(health).map_err(|_| QuartzError::MathOverflow)?;
    Ok(health_u8)
}
