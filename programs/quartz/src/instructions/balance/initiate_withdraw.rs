use anchor_lang::prelude::*;
use solana_program::ed25519_program;
use crate::{
    config::TIME_LOCK_DURATION_SLOTS, 
    state::{Vault, WithdrawOrder}, 
    utils::{allocate_time_lock_owner_payer, allocate_time_lock_program_payer, TimeLock}
};

#[derive(Accounts)]
pub struct InitiateWithdraw<'info> {
    #[account(
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub withdraw_order: Signer<'info>,

    /// CHECK: Checked in handler
    #[account(mut)]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Safe, address is checked
    #[account(address = ed25519_program::ID)]
    pub ed25519_program: UncheckedAccount<'info>
}

pub fn initiate_withdraw_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, InitiateWithdraw<'info>>,
    amount_base_units: u64,
    drift_market_index: u16,
    reduce_only: bool
) -> Result<()> {
    let is_owner_payer = ctx.accounts.time_lock_rent_payer.key().eq(&ctx.accounts.owner.key());

    if is_owner_payer {
        allocate_time_lock_owner_payer(
            &ctx.accounts.owner,
            &ctx.accounts.withdraw_order,
            &ctx.accounts.system_program,
            WithdrawOrder::INIT_SPACE
        )?;
    } else {
        allocate_time_lock_program_payer(
            &ctx.accounts.time_lock_rent_payer.to_account_info(),
            &ctx.accounts.withdraw_order,
            &ctx.accounts.system_program,
            WithdrawOrder::INIT_SPACE
        )?;
    }

    let current_slot = Clock::get()?.slot;
    let release_slot = current_slot + TIME_LOCK_DURATION_SLOTS;

    let signature = [0; 64];

    let withdraw_order = WithdrawOrder {
        time_lock: TimeLock {
            owner: ctx.accounts.owner.key(),
            is_owner_payer,
            release_slot,
            signature
        },
        amount_base_units,
        drift_market_index,
        reduce_only
    };

    withdraw_order.serialize(&mut *ctx.accounts.withdraw_order.data.borrow_mut())?;

    Ok(())
}