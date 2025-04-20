use crate::{
    check,
    config::{QuartzError, DEPOSIT_ADDRESS_SPACE, WSOL_MINT},
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
    cpi::accounts::Withdraw as DriftWithdraw, cpi::withdraw as drift_withdraw, program::Drift,
};
use solana_program::{
    program::{invoke, invoke_signed},
    system_instruction,
};

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
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init_if_needed,
        seeds = [b"withdraw_mule".as_ref(), owner.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = caller,
        token::mint = mint,
        token::authority = vault
    )]
    pub mule: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Checked in handler
    pub owner: UncheckedAccount<'info>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_user: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    pub drift_user_stats: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_state: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    pub drift_signer: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,

    /// CHECK: Safe once key is in withdraw order
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = destination,
        associated_token::token_program = token_program
    )]
    pub destination_spl: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// CHECK: Safe once seeds are correct, deposit address is the pubkey anyone can send tokens to for deposits
    #[account(
        mut,
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
}

/// Permissionless function to fulfil a withdraw order, sending funds to the order's destination
pub fn fulfil_withdraw_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, FulfilWithdraw<'info>>,
) -> Result<()> {
    check!(
        ctx.accounts
            .destination
            .key()
            .eq(&ctx.accounts.withdraw_order.destination),
        QuartzError::InvalidWithdrawDestination
    );

    let (amount_base_units, drift_market_index, reduce_only) = get_order_data(&ctx)?;

    // Validate market index and mint
    let drift_market = get_drift_market(drift_market_index)?;
    check!(
        &ctx.accounts.mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );
    let is_sol = ctx.accounts.mint.key().eq(&WSOL_MINT);

    // First withdraw any idle funds from deposit address
    let funds_to_withdraw_after_idle = transfer_idle_funds(&ctx, is_sol, amount_base_units)?;

    // Withdraw required funds remaining from Drift
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds_vault = &[b"vault", owner.as_ref(), &[vault_bump]];
    let vault_signer = &[&seeds_vault[..]];

    if funds_to_withdraw_after_idle > 0 {
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

        drift_withdraw(
            cpi_ctx,
            drift_market_index,
            funds_to_withdraw_after_idle,
            reduce_only,
        )?;
    }

    // Send mule's balance to destination
    ctx.accounts.mule.reload()?;
    let amount_to_withdraw = ctx.accounts.mule.amount;

    if is_sol {
        // wSOL must be unwrapped and sent as raw SOL, as the destination likely won't have a wSOL ATA
        withdraw_unwrap_lamports(&ctx, vault_signer, amount_to_withdraw)?;
    } else {
        withdraw_spl(&ctx, vault_signer, amount_to_withdraw)?;
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
    )?;

    Ok((amount_base_units, drift_market_index, reduce_only))
}

fn transfer_idle_funds(
    ctx: &Context<FulfilWithdraw>,
    is_sol: bool,
    amount_base_units: u64,
) -> Result<u64> {
    let deposit_address_bump = ctx.bumps.deposit_address;
    let vault = ctx.accounts.vault.key();
    let seeds_deposit_address = &[b"deposit_address", vault.as_ref(), &[deposit_address_bump]];
    let deposit_address_signer = &[&seeds_deposit_address[..]];

    let idle_funds = if is_sol {
        let rent = Rent::get()?;
        let required_rent = rent.minimum_balance(DEPOSIT_ADDRESS_SPACE);
        let available_lamports = ctx
            .accounts
            .deposit_address
            .lamports()
            .checked_sub(required_rent)
            .ok_or(QuartzError::MathOverflow)?;
        let idle_lamports = available_lamports.min(amount_base_units);

        if idle_lamports > 0 {
            invoke_signed(
                &system_instruction::transfer(
                    ctx.accounts.deposit_address.key,
                    ctx.accounts.destination.key,
                    idle_lamports,
                ),
                &[
                    ctx.accounts.deposit_address.to_account_info(),
                    ctx.accounts.destination.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                deposit_address_signer,
            )?;
        }

        idle_lamports
    } else {
        let deposit_address_spl = match ctx.accounts.deposit_address_spl.as_ref() {
            Some(deposit_address_spl) => deposit_address_spl,
            None => return Err(QuartzError::MissingDepositAddressSpl.into()),
        };

        let idle_tokens = deposit_address_spl.amount.min(amount_base_units);

        if idle_tokens > 0 {
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
                idle_tokens,
                ctx.accounts.mint.decimals,
            )?;
        };

        idle_tokens
    };

    let required_funds_remaining = amount_base_units.saturating_sub(idle_funds);
    Ok(required_funds_remaining)
}

fn withdraw_unwrap_lamports(
    ctx: &Context<FulfilWithdraw>,
    vault_signer: &[&[&[u8]]],
    amount_withdrawn: u64,
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

    // Send true_amount_withdrawn to the owner, leaving just the ATA rent remaining
    invoke(
        &system_instruction::transfer(
            ctx.accounts.caller.key,
            ctx.accounts.destination.key,
            amount_withdrawn,
        ),
        &[
            ctx.accounts.caller.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    Ok(())
}

fn withdraw_spl(
    ctx: &Context<FulfilWithdraw>,
    vault_signer: &[&[&[u8]]],
    amount_withdrawn: u64,
) -> Result<()> {
    // Destination SPL is only required if spl_mint is not wSOL
    let destination_spl = match ctx.accounts.destination_spl.as_ref() {
        Some(destination_spl) => destination_spl,
        None => return Err(QuartzError::MissingDestinationSpl.into()),
    };

    // Transfer all tokens from mule to owner_spl
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.mule.to_account_info(),
                to: destination_spl.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            vault_signer,
        ),
        amount_withdrawn,
        ctx.accounts.mint.decimals,
    )?;

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

    Ok(())
}
