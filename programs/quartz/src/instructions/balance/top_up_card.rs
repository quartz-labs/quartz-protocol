use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        TokenInterface, 
        TokenAccount, 
        Mint,
        CloseAccount,
        close_account
    }
};
use drift::{
    program::Drift,
    cpi::withdraw as drift_withdraw, 
    cpi::accounts::Withdraw as DriftWithdraw,
    state::{
        state::State as DriftState, 
        user::{
            User as DriftUser, 
            UserStats as DriftUserStats
        }
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
    config::{QuartzError, DOMAIN_BASE}, 
    state::Vault, 
    utils::get_drift_market
};


#[derive(Accounts)]
pub struct TopUpCard<'info> {
    // --- Standard accounts ---

    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        seeds = [vault.key().as_ref(), usdc_mint.key().as_ref()],
        bump,
        payer = owner,
        token::mint = usdc_mint,
        token::authority = vault
    )]
    pub vault_usdc: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub usdc_mint: Box<InterfaceAccount<'info, Mint>>,


    // --- Drift accounts ---

    #[account(
        mut,
        seeds = [b"user".as_ref(), vault.key().as_ref(), (0u16).to_le_bytes().as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_user: AccountLoader<'info, DriftUser>,
    
    #[account(
        mut,
        seeds = [b"user_stats".as_ref(), vault.key().as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_user_stats: AccountLoader<'info, DriftUserStats>,

    #[account(
        mut,
        seeds = [b"drift_state".as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_state: Box<Account<'info, DriftState>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,
    
    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    pub drift_signer: UncheckedAccount<'info>,

    pub drift_program: Program<'info, Drift>,


    // --- CCTP accounts ---

    /// CHECK: TODO: Ensure this is the correct address
    pub provider_base_address: UncheckedAccount<'info>,

    /// CHECK: TODO: Ensure this is the correct address
    pub quartz_caller_base_address: UncheckedAccount<'info>,

    /// CHECK: TODO: Replace with Quartz rent payer
    pub event_rent_payer: UncheckedAccount<'info>,

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

    /// Account to store MessageSent event data in. Any non-PDA uninitialized address.
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
    let drift_market = get_drift_market(0)?;
    check!(
        &ctx.accounts.usdc_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );
    
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    // Drift Withdraw CPI
    let mut withdraw_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        DriftWithdraw {
            state: ctx.accounts.drift_state.to_account_info(),
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            spot_market_vault: ctx.accounts.spot_market_vault.to_account_info(),
            drift_signer: ctx.accounts.drift_signer.to_account_info(),
            user_token_account: ctx.accounts.vault_usdc.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds
    );

    withdraw_cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    drift_withdraw(withdraw_cpi_ctx, drift_market.market_index, amount_usdc_base_units, false)?;

    // Bridge USDC to Base through Circle CPI
    let bridge_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_messenger_minter_program.to_account_info(), 
        DepositForBurnContext {
            owner: ctx.accounts.vault.to_account_info(),
            event_rent_payer: ctx.accounts.event_rent_payer.to_account_info(),
            sender_authority_pda: ctx.accounts.sender_authority_pda.to_account_info(),
            burn_token_account: ctx.accounts.vault_usdc.to_account_info(),
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
            program: ctx.accounts.token_messenger_minter_program.to_account_info(),
        }, 
        signer_seeds
    );

    let bridge_cpi_params = DepositForBurnWithCallerParams {
        amount: amount_usdc_base_units,
        destination_domain: DOMAIN_BASE,
        mint_recipient: ctx.accounts.provider_base_address.key(),
        destination_caller: ctx.accounts.quartz_caller_base_address.key()
    };

    deposit_for_burn_with_caller(bridge_cpi_ctx, bridge_cpi_params)?;

    // Close vault's ATA
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault_usdc.to_account_info(),
            destination: ctx.accounts.owner.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds
    );
    close_account(cpi_ctx_close)?;

    Ok(())
}