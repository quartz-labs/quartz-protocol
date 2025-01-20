use anchor_lang::{
    prelude::*, 
    solana_program::sysvar::instructions::{
        self,
        load_current_index_checked, 
        load_instruction_at_checked
    },
};
use anchor_spl::token_interface::{
    TransferChecked,
    transfer_checked,
    TokenInterface, 
    TokenAccount, 
    Mint,
    CloseAccount,
    close_account
};
use drift::{
    cpi::{
        accounts::Withdraw as DriftWithdraw, 
        withdraw as drift_withdraw
    },
    program::Drift, 
    state::{
        state::State as DriftState, 
        user::User as DriftUser
    }
};
use pyth_solana_receiver_sdk::price_update::{
    get_feed_id_from_hex, 
    PriceUpdateV2
};
use crate::{
    check, 
    config::{
        QuartzError, 
        COLLATERAL_REPAY_MAX_HEALTH_RESULT_PERCENT, 
        COLLATERAL_REPAY_MAX_SLIPPAGE_BPS, 
        PYTH_MAX_PRICE_AGE_SECONDS
    }, 
    load_mut, 
    state::{DriftMarket, CollateralRepayLedger, Vault}, 
    utils::{
        get_drift_margin_calculation, 
        get_drift_market, 
        get_quartz_account_health, 
        normalize_price_exponents, validate_start_collateral_repay_ix
    }
};

#[derive(Accounts)]
pub struct WithdrawCollateralRepay<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = spl_mint,
        associated_token::authority = caller,
        associated_token::token_program = token_program
    )]
    pub caller_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Can be any account, once it has a Vault
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        seeds = [vault.key().as_ref(), spl_mint.key().as_ref()],
        bump,
        payer = caller,
        token::mint = spl_mint,
        token::authority = vault
    )]
    pub vault_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    pub spl_mint: Box<InterfaceAccount<'info, Mint>>,

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
    pub ledger: Box<Account<'info, CollateralRepayLedger>>
}

pub fn withdraw_collateral_repay_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, WithdrawCollateralRepay<'info>>,
    withdraw_market_index: u16
) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    let vault_seeds = &[
        b"vault",
        owner.as_ref(),
        &[ctx.accounts.vault.bump]
    ];
    let signer_seeds_vault = &[&vault_seeds[..]];

    let withdraw_market = get_drift_market(withdraw_market_index)?;
    check!(
        &ctx.accounts.spl_mint.key().eq(&withdraw_market.mint),
        QuartzError::InvalidMint
    );

    let index: usize = load_current_index_checked(
        &ctx.accounts.instructions.to_account_info()
    )?.into();
    let start_instruction = load_instruction_at_checked(
        index - 3, 
        &ctx.accounts.instructions.to_account_info()
    )?;
    validate_start_collateral_repay_ix(&start_instruction)?;

    // Paranoia check to ensure the vault is empty before withdrawing for amount calculations
    check!(
        ctx.accounts.vault_spl.amount == 0,
        QuartzError::InvalidStartingVaultBalance
    );

    // Calculate withdraw tokens sent to jupiter swap
    let starting_withdraw_spl_balance = ctx.accounts.ledger.withdraw;
    let current_withdraw_spl_balance = ctx.accounts.caller_spl.amount;
    let amount_withdraw_base_units = starting_withdraw_spl_balance - current_withdraw_spl_balance;

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
            user_token_account: ctx.accounts.vault_spl.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds_vault
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    // reduce_only = true to prevent withdrawing more than the collateral position (which would create a new loan)
    drift_withdraw(cpi_ctx, withdraw_market_index, amount_withdraw_base_units, true)?;

    // Validate values of amount deposited and amount withdrawn are within slippage
    ctx.accounts.vault_spl.reload()?;
    let true_amount_withdrawn = ctx.accounts.vault_spl.amount;
    let true_amount_deposited = ctx.accounts.ledger.deposit;

    let deposit_market_index = u16::from_le_bytes(start_instruction.data[8..10].try_into().unwrap());
    let deposit_market = get_drift_market(deposit_market_index)?;

    validate_prices(
        &ctx, 
        true_amount_deposited, 
        true_amount_withdrawn, 
        deposit_market, 
        withdraw_market
    )?;

    // Transfer tokens from vault's ATA to caller's ATA
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            TransferChecked { 
                from: ctx.accounts.vault_spl.to_account_info(), 
                to: ctx.accounts.caller_spl.to_account_info(), 
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.spl_mint.to_account_info(),
            }, 
            signer_seeds_vault
        ),
        true_amount_withdrawn,
        ctx.accounts.spl_mint.decimals
    )?;

    // Close vault's ATA
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault_spl.to_account_info(),
            destination: ctx.accounts.caller.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds_vault
    );
    close_account(cpi_ctx_close)?;

    // Validate account health if the owner isn't the caller
    if !ctx.accounts.owner.key().eq(&ctx.accounts.caller.key()) {
        validate_account_health(
            &ctx, 
            deposit_market_index, 
            withdraw_market.market_index
        )?;
    }

    Ok(())
}

#[inline(never)]
fn validate_prices<'info>(
    ctx: &Context<'_, '_, '_, 'info, WithdrawCollateralRepay<'info>>,
    deposit_amount: u64,
    withdraw_amount: u64,
    deposit_market: &DriftMarket,
    withdraw_market: &DriftMarket
) -> Result<()> {
    // Get the deposit price, assuming worst case of lowest end of confidence interval
    let deposit_feed_id: [u8; 32] = get_feed_id_from_hex(deposit_market.pyth_feed)?;
    let deposit_price = ctx.accounts.deposit_price_update.get_price_no_older_than(
        &Clock::get()?, 
        PYTH_MAX_PRICE_AGE_SECONDS,
        &deposit_feed_id
    )?;
    check!(
        deposit_price.price > 0,
        QuartzError::NegativeOraclePrice
    );
    let deposit_lowest_price_raw = (deposit_price.price as u64).checked_sub(deposit_price.conf)
        .ok_or(QuartzError::NegativeOraclePrice)?;

    // Get the withdraw price, assuming worst case of highest end of confidence interval
    let withdraw_feed_id: [u8; 32] = get_feed_id_from_hex(withdraw_market.pyth_feed)?;
    let withdraw_price = ctx.accounts.withdraw_price_update.get_price_no_older_than(
        &Clock::get()?,
        PYTH_MAX_PRICE_AGE_SECONDS,
        &withdraw_feed_id
    )?;
    check!(
        withdraw_price.price > 0,
        QuartzError::NegativeOraclePrice
    );
    let withdraw_highest_price_raw = (withdraw_price.price as u64) + withdraw_price.conf;

    // Normalize prices to the same exponents
    let (
        deposit_lowest_price_normalized,
        withdraw_highest_price_normalized
    ) = normalize_price_exponents(
        deposit_lowest_price_raw,
        deposit_price.exponent,
        withdraw_highest_price_raw,
        withdraw_price.exponent
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
 
    // Allow for slippage, using integar multiplication to prevent floating point errors
    let slippage_multiplier_deposit: u128 = 100 * 100; // 100% x 100bps
    let slippage_multiplier_withdraw: u128 = slippage_multiplier_deposit - (COLLATERAL_REPAY_MAX_SLIPPAGE_BPS as u128);

    let deposit_slippage_check_value = deposit_value.checked_mul(slippage_multiplier_deposit)
        .ok_or(QuartzError::MathOverflow)?;
    let withdraw_slippage_check_value = withdraw_value.checked_mul(slippage_multiplier_withdraw)
        .ok_or(QuartzError::MathOverflow)?;

    check!(
        deposit_slippage_check_value >= withdraw_slippage_check_value,
        QuartzError::MaxSlippageExceeded
    );

    Ok(())
}

#[inline(never)]
fn validate_account_health<'info>(
    ctx: &Context<'_, '_, 'info, 'info, WithdrawCollateralRepay<'info>>,
    deposit_market_index: u16,
    withdraw_market_index: u16
) -> Result<()> {
    let user = &mut load_mut!(ctx.accounts.drift_user)?;
    let margin_calculation = get_drift_margin_calculation(
        user, 
        &ctx.accounts.drift_state, 
        withdraw_market_index, 
        deposit_market_index,
        &ctx.remaining_accounts
    )?;

    let quartz_account_health = get_quartz_account_health(margin_calculation)?;

    check!(
        quartz_account_health > 0,
        QuartzError::CollateralRepayHealthTooLow
    );

    check!(
        quartz_account_health <= COLLATERAL_REPAY_MAX_HEALTH_RESULT_PERCENT,
        QuartzError::CollateralRepayHealthTooHigh
    );

    Ok(())
}
