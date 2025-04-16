use crate::config::RENT_RECLAIMER;
use anchor_lang::prelude::*;
use message_transmitter::{
    cpi::{accounts::ReclaimEventAccountContext, reclaim_event_account},
    instructions::ReclaimEventAccountParams,
    program::MessageTransmitter,
};

#[derive(Accounts)]
pub struct ReclaimBridgeRent<'info> {
    #[account(
        constraint = rent_reclaimer.key().eq(&RENT_RECLAIMER)
    )]
    pub rent_reclaimer: Signer<'info>,

    /// CHECK: Safe once address is correct
    #[account(
        mut,
        seeds = [b"bridge_rent_payer"],
        bump
    )]
    pub bridge_rent_payer: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    #[account(mut)]
    pub message_transmitter: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    #[account(mut)]
    pub message_sent_event_data: UncheckedAccount<'info>,

    pub cctp_message_transmitter: Program<'info, MessageTransmitter>,
}

pub fn reclaim_bridge_rent_handler(
    ctx: Context<ReclaimBridgeRent>,
    attestation: Vec<u8>,
) -> Result<()> {
    // Reclaims account rent one the bridge for spend is fully processed

    let bridge_rent_payer_bump = ctx.bumps.bridge_rent_payer;
    let bridge_rent_payer_seeds = &[b"bridge_rent_payer".as_ref(), &[bridge_rent_payer_bump]];
    let signer_seeds = &[&bridge_rent_payer_seeds[..]];

    let reclaim_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.cctp_message_transmitter.to_account_info(),
        ReclaimEventAccountContext {
            payee: ctx.accounts.bridge_rent_payer.to_account_info(),
            message_transmitter: ctx.accounts.message_transmitter.to_account_info(),
            message_sent_event_data: ctx.accounts.message_sent_event_data.to_account_info(),
        },
        signer_seeds,
    );

    let reclaim_cpi_params = ReclaimEventAccountParams { attestation };

    reclaim_event_account(reclaim_cpi_ctx, reclaim_cpi_params)?;

    Ok(())
}
