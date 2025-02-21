use anchor_lang::{
    prelude::*, 
    Discriminator
};
use crate::{
    config::RENT_RECLAIMER, 
    state::Vault
};

#[derive(Accounts)]
pub struct ResetSpendLimit<'info> {
    #[account(
        constraint = rent_reclaimer.key().eq(&RENT_RECLAIMER)
    )]
    pub rent_reclaimer: Signer<'info>
}


pub fn reset_spend_limit_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, ResetSpendLimit<'info>>,
) -> Result<()> {
    for account in ctx.remaining_accounts {
        let data = account.try_borrow_data()?;
        if data.len() < 8 || data[0..8] != Vault::DISCRIMINATOR || data.len() != Vault::INIT_SPACE {
            return Err(ErrorCode::AccountNotInitialized.into());
        }

        let mut vault = Account::<Vault>::try_from(account)?;

        vault.spend_limit_per_transaction = 1000_000_000;
        vault.spend_limit_per_timeframe = 0;
        vault.timeframe_in_slots = (1_000 * 60 * 60 * 24) / 400;

        vault.serialize(&mut *account.try_borrow_mut_data()?)?;
    }

    Ok(())
}