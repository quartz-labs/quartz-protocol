use crate::{
    check,
    config::{QuartzError, DEPOSIT_ADDRESS_SPACE, WSOL_MINT},
    state::Vault,
    utils::get_drift_market,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, sync_native, transfer_checked, CloseAccount, Mint, SyncNative, TokenAccount,
        TokenInterface, TransferChecked,
    },
};
use drift::{
    cpi::accounts::Deposit as DriftDeposit, cpi::deposit as drift_deposit, program::Drift,
};
use solana_program::{program::invoke_signed, system_instruction};

#[derive(Accounts)]
pub struct FulfilDeposit<'info> {
    #[account(
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: Safe once seeds are correct, deposit address is the pubkey anyone can send tokens to for deposits
    #[account(
        seeds = [b"deposit_address".as_ref(), vault.key().as_ref()],
        bump
    )]
    pub deposit_address: UncheckedAccount<'info>,

    /// Option because SOL in the deposit_address will be regular lamports, not wSOL
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = deposit_address,
        associated_token::token_program = token_program
    )]
    pub deposit_address_spl: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(
        init_if_needed,
        seeds = [b"deposit_mule".as_ref(), owner.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = caller,
        token::mint = mint,
        token::authority = vault
    )]
    pub mule: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Any account, once it has a vault
    pub owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub caller: Signer<'info>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_user: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_user_stats: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_state: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,
}

/// Anyone can deposit into a Quartz account by sending funds to the deposit_address of that account, this function permissionlessly moves funds from that address into Drift
pub fn fulfil_deposit_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, FulfilDeposit<'info>>,
    drift_market_index: u16,
) -> Result<()> {
    // Validate market index and mint
    let drift_market = get_drift_market(drift_market_index)?;
    check!(
        &ctx.accounts.mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    let deposit_address_bump = ctx.bumps.deposit_address;
    let vault = ctx.accounts.vault.key();
    let seeds_deposit_address = &[b"deposit_address", vault.as_ref(), &[deposit_address_bump]];
    let deposit_address_signer = &[&seeds_deposit_address[..]];

    // Transfer tokens from deposit address ATA to vault's mule
    if ctx.accounts.mint.key().eq(&WSOL_MINT) {
        transfer_deposit_lamports(&ctx, deposit_address_signer)?;
    } else {
        transfer_deposit_spl(&ctx, deposit_address_signer)?;
    }

    // Drift Deposit CPI
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds_vault = &[b"vault", owner.as_ref(), &[vault_bump]];
    let vault_signer = &[&seeds_vault[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        DriftDeposit {
            state: ctx.accounts.drift_state.to_account_info(),
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            spot_market_vault: ctx.accounts.spot_market_vault.to_account_info(),
            user_token_account: ctx.accounts.mule.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        vault_signer,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    // reduce_only = false to allow for a loan position to become a collateral position
    ctx.accounts.mule.reload()?;
    drift_deposit(cpi_ctx, drift_market_index, ctx.accounts.mule.amount, false)?;

    // Close vault's mule
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.mule.to_account_info(),
            destination: ctx.accounts.caller.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        vault_signer,
    );
    close_account(cpi_ctx_close)?;

    Ok(())
}

fn transfer_deposit_spl<'info>(
    ctx: &Context<'_, '_, '_, 'info, FulfilDeposit<'info>>,
    deposit_address_signer: &[&[&[u8]]],
) -> Result<()> {
    let deposit_address_spl = match ctx.accounts.deposit_address_spl.as_ref() {
        Some(deposit_address_spl) => deposit_address_spl,
        None => return Err(QuartzError::MissingDepositAddressSpl.into()),
    };
    let amount_base_units = deposit_address_spl.amount;

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: deposit_address_spl.to_account_info(),
                to: ctx.accounts.mule.to_account_info(),
                authority: ctx.accounts.deposit_address.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            deposit_address_signer,
        ),
        amount_base_units,
        ctx.accounts.mint.decimals,
    )?;

    Ok(())
}

fn transfer_deposit_lamports<'info>(
    ctx: &Context<'_, '_, '_, 'info, FulfilDeposit<'info>>,
    deposit_address_signer: &[&[&[u8]]],
) -> Result<()> {
    let rent = Rent::get()?;
    let required_rent = rent.minimum_balance(DEPOSIT_ADDRESS_SPACE);
    let available_lamports = ctx
        .accounts
        .deposit_address
        .lamports()
        .checked_sub(required_rent)
        .ok_or(QuartzError::MathOverflow)?;

    // Transfer lamports from deposit_address to mule
    invoke_signed(
        &system_instruction::transfer(
            ctx.accounts.deposit_address.key,
            ctx.accounts.mule.to_account_info().key,
            available_lamports,
        ),
        &[
            ctx.accounts.deposit_address.to_account_info(),
            ctx.accounts.mule.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        deposit_address_signer,
    )?;

    // Wrap the lamports
    sync_native(CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SyncNative {
            account: ctx.accounts.mule.to_account_info(),
        },
    ))?;

    Ok(())
}
