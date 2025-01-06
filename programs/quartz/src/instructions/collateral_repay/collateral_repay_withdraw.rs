use anchor_lang::{
    prelude::*, 
    solana_program::{
        instruction::Instruction,
        sysvar::instructions::{
            self,
            load_current_index_checked, 
            load_instruction_at_checked
        }
    }, 
    Discriminator
};
use anchor_spl::{
    token, 
    token_interface::{
        TokenInterface, 
        TokenAccount as TokenAccountInterface, 
        Mint as MintInterface
    }
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
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use crate::{
    check, config::{
        QuartzError, COLLATERAL_REPAY_MAX_HEALTH_RESULT_PERCENT, COLLATERAL_REPAY_MAX_SLIPPAGE_BPS, JUPITER_EXACT_OUT_ROUTE_DISCRIMINATOR, JUPITER_ID
    }, load_mut, state::{DriftMarket, Vault}, utils::{get_drift_margin_calculation, get_drift_market, get_jup_exact_out_route_out_amount, get_quartz_account_health, normalize_price_exponents}
};

#[derive(Accounts)]
pub struct CollateralRepayWithdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), spl_mint.key().as_ref()],
        bump,
        token::mint = spl_mint,
        token::authority = vault
    )]
    pub vault_spl: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    /// CHECK: Can be any account, once it has a Vault
    pub owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = spl_mint,
        associated_token::authority = caller
    )]
    pub caller_spl: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    pub spl_mint: Box<InterfaceAccount<'info, MintInterface>>,

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
}

#[inline(never)]
fn validate_instruction_order<'info>(
    start_instruction: &Instruction,
    swap_instruction: &Instruction,
    deposit_instruction: &Instruction
) -> Result<()> {
    // Check the 1st instruction is collateral_repay_start
    check!(
        start_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        start_instruction.data[..8]
            .eq(&crate::instruction::CollateralRepayStart::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    // Check the 2nd instruction is Jupiter's exact_out_route
    check!(
        swap_instruction.program_id.eq(&JUPITER_ID),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        swap_instruction.data[..8].eq(&JUPITER_EXACT_OUT_ROUTE_DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );
    
    // Check the 3rd instruction is collateral_repay_deposit
    check!(
        deposit_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        deposit_instruction.data[..8]
            .eq(&crate::instruction::CollateralRepayDeposit::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    // This instruction is the 4th instruction

    Ok(())
}

#[inline(never)]
fn validate_user_accounts<'info>(
    ctx: &Context<'_, '_, '_, 'info, CollateralRepayWithdraw<'info>>,
    start_instruction: &Instruction,
    deposit_instruction: &Instruction
) -> Result<()> {
    // Start instruction
    let start_caller = start_instruction.accounts[0].pubkey;
    check!(
        ctx.accounts.caller.key().eq(&start_caller),
        QuartzError::InvalidUserAccounts
    );

    let start_owner = start_instruction.accounts[5].pubkey;
    check!(
        ctx.accounts.owner.key().eq(&start_owner),
        QuartzError::InvalidUserAccounts
    );

    let start_vault = start_instruction.accounts[3].pubkey;
    check!(
        ctx.accounts.vault.key().eq(&start_vault),
        QuartzError::InvalidUserAccounts
    );

    let start_vault_spl = start_instruction.accounts[4].pubkey;
    check!(
        ctx.accounts.vault_spl.key().eq(&start_vault_spl),
        QuartzError::InvalidUserAccounts
    );

    // Deposit instruction
    let deposit_vault = deposit_instruction.accounts[0].pubkey;
    check!(
        ctx.accounts.vault.key().eq(&deposit_vault),
        QuartzError::InvalidUserAccounts
    );

    let deposit_owner = deposit_instruction.accounts[2].pubkey;
    check!(
        ctx.accounts.owner.key().eq(&deposit_owner),
        QuartzError::InvalidUserAccounts
    );

    let deposit_caller = deposit_instruction.accounts[3].pubkey;
    check!(
        ctx.accounts.caller.key().eq(&deposit_caller),
        QuartzError::InvalidUserAccounts
    );

    let deposit_drift_user = deposit_instruction.accounts[6].pubkey;
    check!(
        ctx.accounts.drift_user.key().eq(&deposit_drift_user),
        QuartzError::InvalidUserAccounts
    );

    let deposit_drift_user_stats = deposit_instruction.accounts[7].pubkey;
    check!(
        ctx.accounts.drift_user_stats.key().eq(&deposit_drift_user_stats),
        QuartzError::InvalidUserAccounts
    );

    Ok(())
}

#[inline(never)]
fn validate_prices<'info>(
    ctx: &Context<'_, '_, '_, 'info, CollateralRepayWithdraw<'info>>,
    deposit_amount: u64,
    withdraw_amount: u64,
    deposit_market: &DriftMarket,
    withdraw_market: &DriftMarket
) -> Result<()> {
    // Get the deposit price, assuming worst case of lowest end of confidence interval
    let deposit_feed_id: [u8; 32] = get_feed_id_from_hex(deposit_market.pyth_feed)?;
    let deposit_price = ctx.accounts.deposit_price_update.get_price_no_older_than(
        &Clock::get()?, 
        deposit_market.pyth_max_age_seconds,
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
        withdraw_market.pyth_max_age_seconds,
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
    ctx: &Context<'_, '_, 'info, 'info, CollateralRepayWithdraw<'info>>,
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

pub fn collateral_repay_withdraw_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, CollateralRepayWithdraw<'info>>,
    drift_market_index: u16
) -> Result<()> {
    let index: usize = load_current_index_checked(&ctx.accounts.instructions.to_account_info())?.into();
    let start_instruction = load_instruction_at_checked(index - 3, &ctx.accounts.instructions.to_account_info())?;
    let swap_instruction = load_instruction_at_checked(index - 2, &ctx.accounts.instructions.to_account_info())?;
    let deposit_instruction = load_instruction_at_checked(index - 1, &ctx.accounts.instructions.to_account_info())?;

    validate_instruction_order(&start_instruction, &swap_instruction, &deposit_instruction)?;

    validate_user_accounts(&ctx, &start_instruction, &deposit_instruction)?;

    let withdraw_market = get_drift_market(drift_market_index)?;
    check!(
        &ctx.accounts.spl_mint.key().eq(&withdraw_market.mint),
        QuartzError::InvalidMint
    );

    // Validate mint and ATA are the same as swap
    let swap_source_mint = swap_instruction.accounts[5].pubkey;
    check!(
        swap_source_mint.eq(&ctx.accounts.spl_mint.key()),
        QuartzError::InvalidMint
    );

    let swap_source_token_account = swap_instruction.accounts[2].pubkey;
    check!(
        swap_source_token_account.eq(&ctx.accounts.caller_spl.key()),
        QuartzError::InvalidSourceTokenAccount
    );
    
    // Get amount actually swapped in Jupiter
    let start_balance = u64::from_le_bytes(
        start_instruction.data[8..16].try_into().unwrap()
    );
    let end_balance = ctx.accounts.caller_spl.amount;
    let withdraw_amount = start_balance - end_balance;

    // Validate values of deposit_amount and withdraw_amount are within slippage
    let deposit_amount = get_jup_exact_out_route_out_amount(&swap_instruction)?;
    let deposit_market_index = u16::from_le_bytes(deposit_instruction.data[8..10].try_into().unwrap());
    let deposit_market = get_drift_market(deposit_market_index)?;
    validate_prices(&ctx, deposit_amount, withdraw_amount, deposit_market, withdraw_market)?;

    let owner = ctx.accounts.owner.key();
    let vault_seeds = &[
        b"vault",
        owner.as_ref(),
        &[ctx.accounts.vault.bump]
    ];
    let signer_seeds_vault = &[&vault_seeds[..]];

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
    drift_withdraw(cpi_ctx, drift_market_index, withdraw_amount, true)?;

    // Transfer tokens from vault's ATA to caller's ATA
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            token::Transfer { 
                from: ctx.accounts.vault_spl.to_account_info(), 
                to: ctx.accounts.caller_spl.to_account_info(), 
                authority: ctx.accounts.vault.to_account_info()
            }, 
            signer_seeds_vault
        ),
        withdraw_amount
    )?;

    // Close vault's ATA
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::CloseAccount {
            account: ctx.accounts.vault_spl.to_account_info(),
            destination: ctx.accounts.caller.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds_vault
    );
    token::close_account(cpi_ctx_close)?;

    // Validate account health if the owner isn't the caller

    if !ctx.accounts.owner.key().eq(&ctx.accounts.caller.key()) {
        validate_account_health(&ctx, deposit_market_index, withdraw_market.market_index)?;
    }

    Ok(())
}