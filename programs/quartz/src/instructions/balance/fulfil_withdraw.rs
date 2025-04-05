use crate::{
    check,
    config::{QuartzError, WSOL_MINT},
    state::{Vault, WithdrawOrder},
    utils::{close_time_lock, get_drift_market, validate_time_lock},
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};
use drift::{
    cpi::accounts::Withdraw as DriftWithdraw,
    cpi::withdraw as drift_withdraw,
    program::Drift,
    state::{
        state::State as DriftState,
        user::{User as DriftUser, UserStats as DriftUserStats},
    },
};
use solana_program::{program::invoke, system_instruction};

#[derive(Accounts)]
pub struct FulfilWithdraw<'info> {
    #[account(mut)]
    pub withdraw_order: Box<Account<'info, WithdrawOrder>>,

    /// CHECK: Checked in handler
    #[account(mut)]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        seeds = [b"withdraw_mule".as_ref(), owner.key().as_ref()],
        bump,
        payer = caller,
        token::mint = spl_mint,
        token::authority = vault
    )]
    pub mule: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Any account, once it has a vault and matches the order
    #[account(
        mut,
        constraint = owner.key().eq(&withdraw_order.time_lock.owner)
    )]
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = spl_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program
    )]
    pub owner_spl: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    pub spl_mint: Box<InterfaceAccount<'info, Mint>>,

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

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,
}

pub fn fulfil_withdraw_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, FulfilWithdraw<'info>>,
) -> Result<()> {
    msg!("[1] Start handler");

    let (amount_base_units, drift_market_index, reduce_only) = get_order_data(&ctx)?;

    msg!("[2] Got order data");

    // Validate market index and mint
    let drift_market = get_drift_market(drift_market_index)?;
    check!(
        &ctx.accounts.spl_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let vault_seeds = &[b"vault", owner.as_ref(), &[vault_bump]];
    let vault_signer = &[&vault_seeds[..]];

    msg!("[3] Got vault signer, market index, and mint");

    // Drift Withdraw CPI
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        DriftWithdraw {
            state: ctx.accounts.drift_state.to_account_info(),
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            spot_market_vault: ctx.accounts.spot_market_vault.to_account_info(),
            drift_signer: ctx.accounts.drift_signer.to_account_info(),
            user_token_account: ctx.accounts.mule.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        vault_signer,
    );
    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    drift_withdraw(cpi_ctx, drift_market_index, amount_base_units, reduce_only)?;

    msg!("[4] Drift withdraw CPI complete");

    // Get true amount withdrawn in case reduce_only prevented full withdraw
    ctx.accounts.mule.reload()?;
    let true_amount_withdrawn = ctx.accounts.mule.amount;

    msg!("[5] Got true amount withdrawn");

    if ctx.accounts.spl_mint.key().eq(&WSOL_MINT) {
        transfer_lamports(&ctx, vault_signer, true_amount_withdrawn)?;
    } else {
        transfer_spl(&ctx, vault_signer, true_amount_withdrawn)?;
    }

    Ok(())
}

fn get_order_data(ctx: &Context<FulfilWithdraw>) -> Result<(u64, u16, bool)> {
    validate_time_lock(
        &ctx.accounts.owner.key(),
        &ctx.accounts.withdraw_order.time_lock,
    )?;

    let amount_base_units = ctx.accounts.withdraw_order.amount_base_units;
    let drift_market_index = ctx.accounts.withdraw_order.drift_market_index;
    let reduce_only = ctx.accounts.withdraw_order.reduce_only;

    close_time_lock(
        &ctx.accounts.withdraw_order,
        &ctx.accounts.time_lock_rent_payer.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
    )?;

    Ok((amount_base_units, drift_market_index, reduce_only))
}

fn transfer_lamports(
    ctx: &Context<FulfilWithdraw>,
    vault_signer: &[&[&[u8]]],
    true_amount_withdrawn: u64,
) -> Result<()> {
    // Close wSOL mule, unwrapping all SOL to caller
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

    msg!("[6] Closed mule");

    // Send true_amount_withdrawn to the owner, leaving just the ATA rent remaining
    invoke(
        &system_instruction::transfer(
            ctx.accounts.caller.key,
            ctx.accounts.owner.key,
            true_amount_withdrawn,
        ),
        &[
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.caller.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    msg!("[7] Transferred lamports");

    Ok(())
}

fn transfer_spl(
    ctx: &Context<FulfilWithdraw>,
    vault_signer: &[&[&[u8]]],
    true_amount_withdrawn: u64,
) -> Result<()> {
    let owner_spl = match ctx.accounts.owner_spl.as_ref() {
        Some(owner_spl) => owner_spl,
        None => return Err(QuartzError::InvalidOwnerSplWSOL.into()), // owner_spl is only optional for wSOL
    };

    msg!("[6] Got owner_spl");

    // Transfer all tokens from mule to owner_spl
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.mule.to_account_info(),
                to: owner_spl.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.spl_mint.to_account_info(),
            },
            vault_signer,
        ),
        true_amount_withdrawn,
        ctx.accounts.spl_mint.decimals,
    )?;

    msg!("[7] Transferred tokens");

    // Close mule
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

    msg!("[8] Closed mule");

    Ok(())
}
