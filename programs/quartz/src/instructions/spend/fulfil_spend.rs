use crate::{
    config::{
        DOMAIN_BASE, PROVIDER_BASE_ADDRESS, QUARTZ_CALLER_BASE_ADDRESS, SPEND_CALLER,
        SPEND_FEE_BPS, SPEND_FEE_DESTINATION, TIME_LOCK_RENT_PAYER_SEEDS, USDC_MINT,
    },
    state::SpendHold,
    utils::{close_time_lock, evm_address_to_solana, validate_time_lock},
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use message_transmitter::program::MessageTransmitter;
use token_messenger_minter::{
    cpi::{accounts::DepositForBurnContext, deposit_for_burn_with_caller},
    program::TokenMessengerMinter,
    token_messenger::DepositForBurnWithCallerParams,
};

#[derive(Accounts)]
pub struct FulfilSpend<'info> {
    #[account(
        mut,
        constraint = spend_caller.key().eq(&SPEND_CALLER)
    )]
    pub spend_caller: Signer<'info>,

    #[account(
        mut,
        constraint = usdc_mint.key().eq(&USDC_MINT)
    )]
    pub usdc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Safe once address is correct
    #[account(
        mut,
        seeds = [b"bridge_rent_payer"],
        bump
    )]
    pub bridge_rent_payer: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    pub sender_authority_pda: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    #[account(mut)]
    pub message_transmitter: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    pub token_messenger: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    pub remote_token_messenger: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    pub token_minter: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    #[account(mut)]
    pub local_token: UncheckedAccount<'info>,

    #[account(mut)]
    pub message_sent_event_data: Signer<'info>,

    /// CHECK: This account is passed through to the Circle CPI, which performs the security checks
    pub event_authority: UncheckedAccount<'info>,

    pub message_transmitter_program: Program<'info, MessageTransmitter>,

    pub token_messenger_minter_program: Program<'info, TokenMessengerMinter>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,

    /// CHECK: Address is checked, and owner ensures it's a token account
    #[account(
        mut,
        constraint = spend_fee_destination.key().eq(&SPEND_FEE_DESTINATION),
        constraint = spend_fee_destination.owner.eq(&token_program.key())
    )]
    pub spend_fee_destination: UncheckedAccount<'info>,

    /// CHECK: Safe once seeds are correct
    #[account(
        mut,
        seeds = [TIME_LOCK_RENT_PAYER_SEEDS],
        bump
    )]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    #[account(mut)]
    pub spend_hold: Box<Account<'info, SpendHold>>,

    #[account(
        mut,
        seeds = [b"spend_hold".as_ref(), time_lock_rent_payer.key().as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = time_lock_rent_payer
    )]
    pub spend_hold_vault: Box<InterfaceAccount<'info, TokenAccount>>,
}

#[inline(never)]
pub fn fulfil_spend_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, FulfilSpend<'info>>,
) -> Result<()> {
    let (amount_usdc_base_units, spend_fee) = get_order_data(&ctx)?;

    let time_lock_rent_payer_bump = ctx.bumps.time_lock_rent_payer;
    let time_lock_rent_payer_seeds = &[TIME_LOCK_RENT_PAYER_SEEDS, &[time_lock_rent_payer_bump]];

    let bridge_rent_payer_bump = ctx.bumps.bridge_rent_payer;
    let bridge_rent_payer_seeds = &[b"bridge_rent_payer".as_ref(), &[bridge_rent_payer_bump]];

    let signer_seeds_bridge = &[
        &bridge_rent_payer_seeds[..],
        &time_lock_rent_payer_seeds[..],
    ];

    // If taking a fee, transfer cut of amount from spend_hold_spl to spend_fee_destination
    let fee_amount = if spend_fee {
        let amount = (amount_usdc_base_units * SPEND_FEE_BPS) / 10_000;

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.spend_hold_vault.to_account_info(),
                    to: ctx.accounts.spend_fee_destination.to_account_info(),
                    authority: ctx.accounts.time_lock_rent_payer.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                },
                &[time_lock_rent_payer_seeds],
            ),
            amount,
            ctx.accounts.usdc_mint.decimals,
        )?;

        amount
    } else {
        0
    };

    // Bridge USDC to Base through Circle CPI taking amount from spend_hold_spl
    let bridge_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts
            .token_messenger_minter_program
            .to_account_info(),
        DepositForBurnContext {
            owner: ctx.accounts.time_lock_rent_payer.to_account_info(),
            event_rent_payer: ctx.accounts.bridge_rent_payer.to_account_info(),
            sender_authority_pda: ctx.accounts.sender_authority_pda.to_account_info(),
            burn_token_account: ctx.accounts.spend_hold_vault.to_account_info(),
            message_transmitter: ctx.accounts.message_transmitter.to_account_info(),
            token_messenger: ctx.accounts.token_messenger.to_account_info(),
            remote_token_messenger: ctx.accounts.remote_token_messenger.to_account_info(),
            token_minter: ctx.accounts.token_minter.to_account_info(),
            local_token: ctx.accounts.local_token.to_account_info(),
            burn_token_mint: ctx.accounts.usdc_mint.to_account_info(),
            message_sent_event_data: ctx.accounts.message_sent_event_data.to_account_info(),
            message_transmitter_program: ctx.accounts.message_transmitter_program.to_account_info(),
            token_messenger_minter_program: ctx
                .accounts
                .token_messenger_minter_program
                .to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            event_authority: ctx.accounts.event_authority.to_account_info(),
            program: ctx
                .accounts
                .token_messenger_minter_program
                .to_account_info(),
        },
        signer_seeds_bridge,
    );

    let provider_base_address_solana = evm_address_to_solana(PROVIDER_BASE_ADDRESS)?;
    let quartz_caller_base_address_solana = evm_address_to_solana(QUARTZ_CALLER_BASE_ADDRESS)?;

    let amount_to_bridge = amount_usdc_base_units - fee_amount;
    let bridge_cpi_params = DepositForBurnWithCallerParams {
        amount: amount_to_bridge,
        destination_domain: DOMAIN_BASE,
        mint_recipient: provider_base_address_solana,
        destination_caller: quartz_caller_base_address_solana,
    };

    deposit_for_burn_with_caller(bridge_cpi_ctx, bridge_cpi_params)?;

    Ok(())
}

#[inline(never)]
pub fn get_order_data(ctx: &Context<FulfilSpend>) -> Result<(u64, bool)> {
    validate_time_lock(
        &ctx.accounts.spend_hold.time_lock.owner, // Don't care who the owner is
        &ctx.accounts.spend_hold.time_lock,
    )?;

    let amount_usdc_base_units = ctx.accounts.spend_hold.amount_usdc_base_units;
    let spend_fee = ctx.accounts.spend_hold.spend_fee;

    close_time_lock(
        &ctx.accounts.spend_hold,
        &ctx.accounts.time_lock_rent_payer.to_account_info(),
        &ctx.accounts.time_lock_rent_payer.to_account_info(), // Don't care who the owner is
    )?;

    Ok((amount_usdc_base_units, spend_fee))
}
