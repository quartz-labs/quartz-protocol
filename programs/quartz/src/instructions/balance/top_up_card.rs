use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        TokenInterface, 
        TokenAccount, 
        Mint,
    }
};
use token_messenger_minter::{
    cpi::{
        accounts::DepositForBurnContext, 
        deposit_for_burn_with_caller
    },
    program::TokenMessengerMinter, 
    token_messenger::DepositForBurnWithCallerParams
};
use message_transmitter::program::MessageTransmitter;
use crate::{
    check, 
    config::{QuartzError, DOMAIN_BASE, PROVIDER_BASE_ADDRESS, QUARTZ_CALLER_BASE_ADDRESS, USDC_MARKET_INDEX}, 
    state::Vault, 
    utils::{evm_address_to_solana, get_drift_market}
};


#[derive(Accounts)]
pub struct TopUpCard<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program
    )]
    pub owner_usdc: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
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
}


pub fn top_up_card_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, TopUpCard<'info>>, 
    amount_usdc_base_units: u64
) -> Result<()> {
    // Validate USDC market index and mint
    let drift_market = get_drift_market(USDC_MARKET_INDEX)?;
    check!(
        &ctx.accounts.usdc_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    // Bridge USDC to Base through Circle CPI
    let bridge_rent_payer_bump = ctx.bumps.bridge_rent_payer;
    let bridge_rent_payer_seeds = &[
        b"bridge_rent_payer".as_ref(),
        &[bridge_rent_payer_bump]
    ];
    let signer_seeds = &[&bridge_rent_payer_seeds[..]];

    let bridge_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_messenger_minter_program.to_account_info(), 
        DepositForBurnContext {
            owner: ctx.accounts.owner.to_account_info(),
            event_rent_payer: ctx.accounts.bridge_rent_payer.to_account_info(),
            sender_authority_pda: ctx.accounts.sender_authority_pda.to_account_info(),
            burn_token_account: ctx.accounts.owner_usdc.to_account_info(),
            message_transmitter: ctx.accounts.message_transmitter.to_account_info(),
            token_messenger: ctx.accounts.token_messenger.to_account_info(),
            remote_token_messenger: ctx.accounts.remote_token_messenger.to_account_info(),
            token_minter: ctx.accounts.token_minter.to_account_info(),
            local_token: ctx.accounts.local_token.to_account_info(),
            burn_token_mint: ctx.accounts.usdc_mint.to_account_info(),
            message_sent_event_data: ctx.accounts.message_sent_event_data.to_account_info(),
            message_transmitter_program: ctx.accounts.message_transmitter_program.to_account_info(),
            token_messenger_minter_program: ctx.accounts.token_messenger_minter_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            event_authority: ctx.accounts.event_authority.to_account_info(),
            program: ctx.accounts.token_messenger_minter_program.to_account_info()
        }, 
        signer_seeds
    );

    let provider_base_address_solana = evm_address_to_solana(PROVIDER_BASE_ADDRESS)?;
    let quartz_caller_base_address_solana = evm_address_to_solana(QUARTZ_CALLER_BASE_ADDRESS)?;
    let bridge_cpi_params = DepositForBurnWithCallerParams {
        amount: amount_usdc_base_units,
        destination_domain: DOMAIN_BASE,
        mint_recipient: provider_base_address_solana,
        destination_caller: quartz_caller_base_address_solana
    };

    deposit_for_burn_with_caller(bridge_cpi_ctx, bridge_cpi_params)?;

    Ok(())
}