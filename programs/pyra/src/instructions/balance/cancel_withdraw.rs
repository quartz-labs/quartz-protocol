use crate::{check, config::PyraError, state::WithdrawOrder, utils::close_time_lock};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CancelWithdraw<'info> {
    #[account(mut)]
    pub withdraw_order: Box<Account<'info, WithdrawOrder>>,

    pub owner: Signer<'info>,

    /// CHECK: Checked in handler
    #[account(mut)]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Creates a time locked withdraw order, which can be fulfilled permissionlessly once the time lock has expired. Time locks prevent edge cases of double spend with the Quartz card.
pub fn cancel_withdraw_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, CancelWithdraw<'info>>,
) -> Result<()> {
    check!(
        ctx.accounts
            .withdraw_order
            .time_lock
            .owner
            .eq(&ctx.accounts.owner.key()),
        PyraError::InvalidTimeLockOwner
    );

    close_time_lock(
        &ctx.accounts.withdraw_order,
        &ctx.accounts.time_lock_rent_payer.to_account_info(),
    )?;

    Ok(())
}
