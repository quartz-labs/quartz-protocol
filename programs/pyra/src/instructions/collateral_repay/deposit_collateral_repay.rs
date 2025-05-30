use crate::{
    check,
    config::PyraError,
    load_mut,
    state::{CollateralRepayLedger, Vault},
    utils::{get_account_health, get_drift_market, validate_start_collateral_repay_ix},
};
use anchor_lang::{
    prelude::*,
    solana_program::sysvar::instructions::{
        self, load_current_index_checked, load_instruction_at_checked,
    },
};
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
    TransferChecked,
};
use drift::{
    cpi::{accounts::Deposit as DriftDeposit, deposit as drift_deposit},
    program::Drift,
    state::{state::State as DriftState, user::User as DriftUser},
};

#[derive(Accounts)]
pub struct DepositCollateralRepay<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = caller,
        associated_token::token_program = token_program
    )]
    pub caller_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Can be any account, once it has a Vault
    pub owner: UncheckedAccount<'info>,

    #[account(
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init_if_needed,
        seeds = [b"collateral_repay_mule".as_ref(), owner.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = caller,
        token::mint = mint,
        token::authority = vault
    )]
    pub mule: Box<InterfaceAccount<'info, TokenAccount>>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,

    // Checked here as required for health calculations
    #[account(
        mut,
        seeds = [b"user".as_ref(), vault.key().as_ref(), (0u16).to_le_bytes().as_ref()],
        seeds::program = drift_program.key(),
        bump
    )]
    pub drift_user: AccountLoader<'info, DriftUser>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_user_stats: UncheckedAccount<'info>,

    /// CHECK: Seeds don't need to be checked on this account as the Drift CPI performs the checks
    #[account(mut)]
    pub drift_state: Box<Account<'info, DriftState>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,

    /// CHECK: Account is safe once address is correct
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"collateral_repay_ledger".as_ref(), owner.key().as_ref()],
        bump
    )]
    pub ledger: Box<Account<'info, CollateralRepayLedger>>,
}

/// Second collateral repay instruction, takes place after swap. Deposits tokens into Drift to pay off loan.
pub fn deposit_collateral_repay_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, DepositCollateralRepay<'info>>,
    deposit_market_index: u16,
) -> Result<()> {
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[b"vault", owner.as_ref(), &[vault_bump]];
    let signer_seeds = &[&seeds[..]];

    let deposit_market = get_drift_market(deposit_market_index)?;
    check!(
        &ctx.accounts.mint.key().eq(&deposit_market.mint),
        PyraError::InvalidMint
    );

    let index: usize =
        load_current_index_checked(&ctx.accounts.instructions.to_account_info())?.into();
    let current_instruction =
        load_instruction_at_checked(index, &ctx.accounts.instructions.to_account_info())?;
    let start_instruction =
        load_instruction_at_checked(index - 2, &ctx.accounts.instructions.to_account_info())?;
    validate_start_collateral_repay_ix(&current_instruction, &start_instruction)?;

    // Validate auto repay threshold if owner hasn't signed
    if !ctx.accounts.owner.is_signer {
        let withdraw_instruction =
            load_instruction_at_checked(index + 1, &ctx.accounts.instructions.to_account_info())?;
        let withdraw_market_index = u16::from_le_bytes(
            withdraw_instruction.data[8..10]
                .try_into()
                .map_err(|_| PyraError::FailedToDeserializeMarketIndex)?,
        );

        validate_health(&ctx, deposit_market_index, withdraw_market_index)?;
    }

    // Calculate deposit tokens received from Jupiter swap
    let starting_deposit_spl_balance = ctx.accounts.ledger.deposit;
    let current_deposit_spl_balance = ctx.accounts.caller_spl.amount;
    let amount_deposit_base_units = current_deposit_spl_balance
        .checked_sub(starting_deposit_spl_balance)
        .ok_or(PyraError::MathOverflow)?;

    // Transfer tokens from caller's ATA to mule
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.caller_spl.to_account_info(),
                to: ctx.accounts.mule.to_account_info(),
                authority: ctx.accounts.caller.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        amount_deposit_base_units,
        ctx.accounts.mint.decimals,
    )?;

    // Drift Deposit CPI
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
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    // reduce_only = true means that the caller can not deposit more than the user's borrowed position / create a collateral position
    drift_deposit(
        cpi_ctx,
        deposit_market_index,
        amount_deposit_base_units,
        true,
    )?;

    // Return any remaining balance (in case reduce_only prevented full deposit)
    ctx.accounts.mule.reload()?;
    let remaining_balance = ctx.accounts.mule.amount;
    if remaining_balance > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.mule.to_account_info(),
                    to: ctx.accounts.caller_spl.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                signer_seeds,
            ),
            remaining_balance,
            ctx.accounts.mint.decimals,
        )?;
    }

    // Close mule
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.mule.to_account_info(),
            destination: ctx.accounts.caller.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds,
    );
    close_account(cpi_ctx_close)?;

    // Log the amount of tokens deposited to the ledger
    let true_amount_deposited = amount_deposit_base_units.saturating_sub(remaining_balance);
    ctx.accounts.ledger.deposit = true_amount_deposited;

    Ok(())
}

#[inline(never)]
fn validate_health<'info>(
    ctx: &Context<'_, '_, 'info, 'info, DepositCollateralRepay<'info>>,
    deposit_market_index: u16,
    withdraw_market_index: u16,
) -> Result<()> {
    let user = &mut load_mut!(ctx.accounts.drift_user)?;
    let health = get_account_health(
        user,
        &ctx.accounts.drift_state,
        withdraw_market_index,
        deposit_market_index,
        ctx.remaining_accounts,
    )?;

    check!(health == 0, PyraError::AutoRepayThresholdNotReached);

    Ok(())
}
