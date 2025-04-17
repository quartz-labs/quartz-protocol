use crate::{
    config::{QuartzError, ANCHOR_DISCRIMINATOR, TIME_LOCK_DURATION_SLOTS},
    state::{SpendLimitsOrder, TimeLock, Vault},
    utils::{allocate_time_lock_owner_payer, allocate_time_lock_program_payer},
};
use anchor_lang::{prelude::*, Discriminator};

#[derive(Accounts)]
pub struct InitiateSpendLimits<'info> {
    #[account(
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub spend_limits_order: Signer<'info>,

    /// CHECK: Checked in handler
    #[account(mut)]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initiate_spend_limits_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, InitiateSpendLimits<'info>>,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64,
) -> Result<()> {
    let is_owner_payer = ctx
        .accounts
        .time_lock_rent_payer
        .key()
        .eq(&ctx.accounts.owner.key());

    if is_owner_payer {
        allocate_time_lock_owner_payer(
            &ctx.accounts.owner,
            &ctx.accounts.spend_limits_order,
            &ctx.accounts.system_program,
            SpendLimitsOrder::INIT_SPACE,
        )?;
    } else {
        allocate_time_lock_program_payer(
            &ctx.accounts.time_lock_rent_payer.to_account_info(),
            &ctx.accounts.spend_limits_order,
            &ctx.accounts.system_program,
            SpendLimitsOrder::INIT_SPACE,
        )?;
    }

    let current_slot = Clock::get()?.slot;
    let release_slot = current_slot
        .checked_add(TIME_LOCK_DURATION_SLOTS)
        .ok_or(QuartzError::MathOverflow)?;

    let spend_limits_order_data = SpendLimitsOrder {
        time_lock: TimeLock {
            owner: ctx.accounts.owner.key(),
            is_owner_payer,
            release_slot,
        },
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        timeframe_in_seconds,
        next_timeframe_reset_timestamp,
    };
    let spend_limits_order_data_vec = spend_limits_order_data.try_to_vec()?;

    let mut data = ctx.accounts.spend_limits_order.try_borrow_mut_data()?;
    data[..ANCHOR_DISCRIMINATOR].copy_from_slice(&SpendLimitsOrder::DISCRIMINATOR);
    data[ANCHOR_DISCRIMINATOR..].copy_from_slice(&spend_limits_order_data_vec[..]);

    Ok(())
}
