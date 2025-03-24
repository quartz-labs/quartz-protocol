use anchor_lang::prelude::*;
use solana_program::ed25519_program;
use crate::{
    config::TIME_LOCK_DURATION_SLOTS, 
    state::{SpendLimitsOrder, Vault}, 
    utils::{allocate_time_lock_owner_payer, allocate_time_lock_program_payer, TimeLock}
};

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

    /// CHECK: Safe, address is checked
    #[account(address = ed25519_program::ID)]
    pub ed25519_program: UncheckedAccount<'info>
}

pub fn initiate_spend_limits_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, InitiateSpendLimits<'info>>,
    spend_limit_per_transaction: u64,
    spend_limit_per_timeframe: u64,
    timeframe_in_seconds: u64,
    next_timeframe_reset_timestamp: u64
) -> Result<()> {
    let is_owner_payer = ctx.accounts.time_lock_rent_payer.key().eq(&ctx.accounts.owner.key());

    if is_owner_payer {
        allocate_time_lock_owner_payer(
            &ctx.accounts.owner,
            &ctx.accounts.spend_limits_order,
            &ctx.accounts.system_program,
            SpendLimitsOrder::INIT_SPACE
        )?;
    } else {
        allocate_time_lock_program_payer(
            &ctx.accounts.time_lock_rent_payer.to_account_info(),
            &ctx.accounts.spend_limits_order,
            &ctx.accounts.system_program,
            SpendLimitsOrder::INIT_SPACE
        )?;
    }

    let current_slot = Clock::get()?.slot;
    let release_slot = current_slot + TIME_LOCK_DURATION_SLOTS;

    let signature = [0; 64];

    let spend_limits_order = SpendLimitsOrder {
        time_lock: TimeLock {
            owner: ctx.accounts.owner.key(),
            is_owner_payer,
            release_slot,
            signature
        },
        spend_limit_per_transaction,
        spend_limit_per_timeframe,
        timeframe_in_seconds,
        next_timeframe_reset_timestamp
    };

    spend_limits_order.serialize(&mut *ctx.accounts.spend_limits_order.data.borrow_mut())?;

    Ok(())
}