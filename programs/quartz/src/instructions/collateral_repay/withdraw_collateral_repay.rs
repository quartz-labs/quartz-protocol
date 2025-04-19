use crate::{
    check,
    config::{
        DriftMarket, QuartzError, AUTO_REPAY_MAX_HEALTH_RESULT_PERCENT,
        AUTO_REPAY_MAX_SLIPPAGE_BPS, PYTH_MAX_PRICE_AGE_SECONDS,
    },
    load_mut,
    state::{CollateralRepayLedger, Vault},
    utils::{
        get_account_health, get_drift_market, normalize_price_exponents,
        validate_start_collateral_repay_ix,
    },
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
    cpi::{accounts::Withdraw as DriftWithdraw, withdraw as drift_withdraw},
    program::Drift,
    state::{state::State as DriftState, user::User as DriftUser},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

#[derive(Accounts)]
pub struct WithdrawCollateralRepay<'info> {
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

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub drift_state: Box<Account<'info, DriftState>>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    pub drift_signer: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,

    pub deposit_price_update: Box<Account<'info, PriceUpdateV2>>,

    pub withdraw_price_update: Box<Account<'info, PriceUpdateV2>>,

    /// CHECK: Account is safe once address is correct
    #[account(address = instructions::ID)]
    instructions: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"collateral_repay_ledger".as_ref(), owner.key().as_ref()],
        bump,
        close = caller
    )]
    pub ledger: Box<Account<'info, CollateralRepayLedger>>,
}

/// Third collateral repay instruction, takes place after deposit. Withdraws collateral from Drift, checking values of deposit and withdraw are below slippage.
pub fn withdraw_collateral_repay_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, WithdrawCollateralRepay<'info>>,
    withdraw_market_index: u16,
) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    let vault_seeds = &[b"vault", owner.as_ref(), &[ctx.accounts.vault.bump]];
    let signer_seeds_vault = &[&vault_seeds[..]];

    let withdraw_market = get_drift_market(withdraw_market_index)?;
    check!(
        &ctx.accounts.mint.key().eq(&withdraw_market.mint),
        QuartzError::InvalidMint
    );

    let index: usize =
        load_current_index_checked(&ctx.accounts.instructions.to_account_info())?.into();
    let current_instruction =
        load_instruction_at_checked(index, &ctx.accounts.instructions.to_account_info())?;
    let start_instruction =
        load_instruction_at_checked(index - 3, &ctx.accounts.instructions.to_account_info())?;
    validate_start_collateral_repay_ix(&current_instruction, &start_instruction)?;

    // Paranoia check to ensure the vault is empty before withdrawing for amount calculations
    check!(
        ctx.accounts.mule.amount == 0,
        QuartzError::InvalidStartingVaultBalance
    );

    // Calculate withdraw tokens sent to jupiter swap
    let starting_withdraw_spl_balance = ctx.accounts.ledger.withdraw;
    let current_withdraw_spl_balance = ctx.accounts.caller_spl.amount;
    let amount_withdraw_base_units = starting_withdraw_spl_balance
        .checked_sub(current_withdraw_spl_balance)
        .ok_or(QuartzError::MathOverflow)?;

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
        signer_seeds_vault,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    // reduce_only = true to prevent withdrawing more than the collateral position (which would create a new loan)
    drift_withdraw(
        cpi_ctx,
        withdraw_market_index,
        amount_withdraw_base_units,
        true,
    )?;

    // Validate values of amount deposited and amount withdrawn are within slippage
    ctx.accounts.mule.reload()?;
    let true_amount_withdrawn = ctx.accounts.mule.amount;
    let true_amount_deposited = ctx.accounts.ledger.deposit;

    let deposit_instruction =
        load_instruction_at_checked(index - 1, &ctx.accounts.instructions.to_account_info())?;
    let deposit_market_index = u16::from_le_bytes(
        deposit_instruction.data[8..10]
            .try_into()
            .map_err(|_| QuartzError::FailedToDeserializeMarketIndex)?,
    );
    let deposit_market = get_drift_market(deposit_market_index)?;

    validate_prices(
        &ctx,
        true_amount_deposited,
        true_amount_withdrawn,
        deposit_market,
        withdraw_market,
    )?;

    // Transfer tokens from mule to caller's ATA
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.mule.to_account_info(),
                to: ctx.accounts.caller_spl.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            signer_seeds_vault,
        ),
        true_amount_withdrawn,
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
        signer_seeds_vault,
    );
    close_account(cpi_ctx_close)?;

    // Validate auto repay threshold if owner hasn't signed
    if !ctx.accounts.owner.is_signer {
        validate_health(&ctx, deposit_market_index, withdraw_market.market_index)?;
    }

    Ok(())
}

/// Takes the deposit and withdraw amounts, their prices, and validates that the withdraw amount is within slippage of the deposit amount
#[inline(never)]
fn validate_prices<'info>(
    ctx: &Context<'_, '_, '_, 'info, WithdrawCollateralRepay<'info>>,
    deposit_amount: u64,
    withdraw_amount: u64,
    deposit_market: &DriftMarket,
    withdraw_market: &DriftMarket,
) -> Result<()> {
    // Get the deposit price, assuming worst case of lowest end of confidence interval
    let deposit_feed_id: [u8; 32] = get_feed_id_from_hex(deposit_market.pyth_feed)?;
    let deposit_price = ctx.accounts.deposit_price_update.get_price_no_older_than(
        &Clock::get()?,
        PYTH_MAX_PRICE_AGE_SECONDS,
        &deposit_feed_id,
    )?;
    check!(deposit_price.price > 0, QuartzError::NegativeOraclePrice);
    let deposit_lowest_price = u64::try_from(deposit_price.price)
        .map_err(|_| QuartzError::MathOverflow)?
        .checked_sub(deposit_price.conf)
        .ok_or(QuartzError::NegativeOraclePrice)?;

    // Get the withdraw price, assuming worst case of highest end of confidence interval
    let withdraw_feed_id: [u8; 32] = get_feed_id_from_hex(withdraw_market.pyth_feed)?;
    let withdraw_price = ctx.accounts.withdraw_price_update.get_price_no_older_than(
        &Clock::get()?,
        PYTH_MAX_PRICE_AGE_SECONDS,
        &withdraw_feed_id,
    )?;
    check!(withdraw_price.price > 0, QuartzError::NegativeOraclePrice);
    let withdraw_highest_price = u64::try_from(withdraw_price.price)
        .map_err(|_| QuartzError::MathOverflow)?
        .checked_add(withdraw_price.conf)
        .ok_or(QuartzError::MathOverflow)?;

    // Normalize prices to the same exponents
    let (deposit_lowest_price_normalized, withdraw_highest_price_normalized) =
        normalize_price_exponents(
            deposit_lowest_price as u128,
            deposit_price.exponent,
            withdraw_highest_price as u128,
            withdraw_price.exponent,
        )?;

    // Normalize amounts to the same decimals (base units per token)
    let deposit_amount_normalized: u128 = (deposit_amount as u128)
        .checked_mul(withdraw_market.base_units_per_token as u128)
        .ok_or(QuartzError::MathOverflow)?;
    let withdraw_amount_normalized: u128 = (withdraw_amount as u128)
        .checked_mul(deposit_market.base_units_per_token as u128)
        .ok_or(QuartzError::MathOverflow)?;

    // Calculate values
    let deposit_value: u128 = deposit_amount_normalized
        .checked_mul(deposit_lowest_price_normalized)
        .ok_or(QuartzError::MathOverflow)?;
    let withdraw_value: u128 = withdraw_amount_normalized
        .checked_mul(withdraw_highest_price_normalized)
        .ok_or(QuartzError::MathOverflow)?;

    // Sanity check on slippage
    const HARD_MAX_SLIPPAGE_BPS: u16 = 500;
    check!(
        AUTO_REPAY_MAX_SLIPPAGE_BPS <= HARD_MAX_SLIPPAGE_BPS,
        QuartzError::InvalidSlippageBPS
    );

    // Allow for slippage, using integar multiplication to prevent floating point errors
    let slippage_multiplier_deposit: u128 = 100 * 100; // 100% x 100bps
    let slippage_multiplier_withdraw: u128 = slippage_multiplier_deposit
        .checked_sub(AUTO_REPAY_MAX_SLIPPAGE_BPS as u128)
        .ok_or(QuartzError::MathOverflow)?;

    let deposit_slippage_check_value = deposit_value
        .checked_mul(slippage_multiplier_deposit)
        .ok_or(QuartzError::MathOverflow)?;
    let withdraw_slippage_check_value = withdraw_value
        .checked_mul(slippage_multiplier_withdraw)
        .ok_or(QuartzError::MathOverflow)?;

    check!(
        deposit_slippage_check_value >= withdraw_slippage_check_value,
        QuartzError::MaxSlippageExceeded
    );

    Ok(())
}

#[inline(never)]
fn validate_health<'info>(
    ctx: &Context<'_, '_, 'info, 'info, WithdrawCollateralRepay<'info>>,
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

    check!(health > 0, QuartzError::AutoRepayNotEnoughSold);

    check!(
        health <= AUTO_REPAY_MAX_HEALTH_RESULT_PERCENT,
        QuartzError::AutoRepayTooMuchSold
    );

    Ok(())
}
