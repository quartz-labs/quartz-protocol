use crate::{
    check,
    config::{
        QuartzError, SPEND_CALLER, SPEND_HOLD_DURATION_SLOTS, TIME_LOCK_RENT_PAYER_SEEDS,
        USDC_MARKET_INDEX,
    },
    state::{SpendHold, Vault},
    utils::{allocate_time_lock_program_payer, get_drift_market, TimeLock},
};
use anchor_lang::{prelude::*, Discriminator};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
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

#[event_cpi]
#[derive(Accounts)]
pub struct InitiateSpend<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: Can be any account, once it has a Vault
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = spend_caller.key().eq(&SPEND_CALLER)
    )]
    pub spend_caller: Signer<'info>,

    #[account(mut)]
    pub usdc_mint: Box<InterfaceAccount<'info, Mint>>,

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

    /// CHECK: Safe once seeds are correct
    #[account(
        mut,
        seeds = [TIME_LOCK_RENT_PAYER_SEEDS],
        bump
    )]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    #[account(mut)]
    pub spend_hold: Signer<'info>,

    #[account(
        init_if_needed,
        seeds = [b"spend_hold".as_ref(), time_lock_rent_payer.key().as_ref()],
        bump,
        payer = time_lock_rent_payer,
        token::mint = usdc_mint,
        token::authority = time_lock_rent_payer
    )]
    pub spend_hold_vault: Box<InterfaceAccount<'info, TokenAccount>>,
}

pub fn initiate_spend_handler<'info>(
    mut ctx: Context<'_, '_, '_, 'info, InitiateSpend<'info>>,
    amount_usdc_base_units: u64,
    spend_fee: bool,
) -> Result<()> {
    initiate_time_lock(&mut ctx, amount_usdc_base_units, spend_fee)?;

    // Manually check mint in handler to avoid Anchor stack overflow
    let drift_market = get_drift_market(USDC_MARKET_INDEX)?;
    check!(
        &ctx.accounts.usdc_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    process_spend_limits(&mut ctx, amount_usdc_base_units)?;

    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[b"vault", owner.as_ref(), &[vault_bump]];
    let signer_seeds = &[&seeds[..]];

    // Use Drift Withdraw CPI to transfer USDC to spend_hold_vault
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.drift_program.to_account_info(),
        DriftWithdraw {
            state: ctx.accounts.drift_state.to_account_info(),
            user: ctx.accounts.drift_user.to_account_info(),
            user_stats: ctx.accounts.drift_user_stats.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
            spot_market_vault: ctx.accounts.spot_market_vault.to_account_info(),
            drift_signer: ctx.accounts.drift_signer.to_account_info(),
            user_token_account: ctx.accounts.spend_hold_vault.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    drift_withdraw(cpi_ctx, USDC_MARKET_INDEX, amount_usdc_base_units, false)?;

    Ok(())
}

fn initiate_time_lock<'info>(
    ctx: &mut Context<'_, '_, '_, 'info, InitiateSpend<'info>>,
    amount_usdc_base_units: u64,
    spend_fee: bool,
) -> Result<()> {
    allocate_time_lock_program_payer(
        &ctx.accounts.time_lock_rent_payer.to_account_info(),
        &ctx.accounts.spend_hold,
        &ctx.accounts.system_program,
        SpendHold::INIT_SPACE,
    )?;

    let current_slot = Clock::get()?.slot;
    let release_slot = current_slot + SPEND_HOLD_DURATION_SLOTS;

    let spend_hold_data = SpendHold {
        time_lock: TimeLock {
            owner: ctx.accounts.owner.key(),
            is_owner_payer: false,
            release_slot,
        },
        amount_usdc_base_units,
        spend_fee,
    };

    let mut data = ctx.accounts.spend_hold.try_borrow_mut_data()?;
    data[..8].copy_from_slice(&SpendHold::DISCRIMINATOR);
    spend_hold_data.serialize(&mut &mut data[8..])?;

    Ok(())
}

fn process_spend_limits<'info>(
    ctx: &mut Context<'_, '_, '_, 'info, InitiateSpend<'info>>,
    amount_usdc_base_units: u64,
) -> Result<()> {
    let current_timestamp_raw = Clock::get()?.unix_timestamp;
    check!(current_timestamp_raw > 0, QuartzError::InvalidTimestamp);
    let current_timestamp = current_timestamp_raw as u64;

    if ctx.accounts.vault.spend_limit_per_transaction < amount_usdc_base_units {
        let error_code = QuartzError::InsufficientTransactionSpendLimit;
        anchor_lang::prelude::msg!(
            "Error \"{}\" ({} < {}) thrown at {}:{}",
            error_code,
            ctx.accounts.vault.spend_limit_per_transaction,
            amount_usdc_base_units,
            file!(),
            line!()
        );
        return Err(error_code.into());
    }

    if ctx.accounts.vault.timeframe_in_seconds == 0 {
        let error_code = QuartzError::InsufficientTimeframeSpendLimit;
        anchor_lang::prelude::msg!(
            "Error \"{}\" (timeframe_in_seconds == 0) thrown at {}:{}",
            error_code,
            file!(),
            line!()
        );
        return Err(error_code.into());
    }

    // If the timeframe has elapsed, incrememt it and reset spend limit
    if current_timestamp >= ctx.accounts.vault.next_timeframe_reset_timestamp {
        let overflow = current_timestamp - ctx.accounts.vault.next_timeframe_reset_timestamp;
        let overflow_in_timeframes = overflow / ctx.accounts.vault.timeframe_in_seconds;
        let seconds_to_add = (overflow_in_timeframes + 1)
            .checked_mul(ctx.accounts.vault.timeframe_in_seconds)
            .ok_or(QuartzError::MathOverflow)?;

        ctx.accounts.vault.next_timeframe_reset_timestamp = ctx
            .accounts
            .vault
            .next_timeframe_reset_timestamp
            .checked_add(seconds_to_add)
            .ok_or(QuartzError::MathOverflow)?;
        ctx.accounts.vault.remaining_spend_limit_per_timeframe =
            ctx.accounts.vault.spend_limit_per_timeframe;
    }

    if ctx.accounts.vault.remaining_spend_limit_per_timeframe < amount_usdc_base_units {
        let error_code = QuartzError::InsufficientTimeframeSpendLimit;
        anchor_lang::prelude::msg!(
            "Error \"{}\" ({} < {}) thrown at {}:{}",
            error_code,
            ctx.accounts.vault.remaining_spend_limit_per_timeframe,
            amount_usdc_base_units,
            file!(),
            line!()
        );
        return Err(error_code.into());
    }

    // Adjust remaining spend limit
    ctx.accounts.vault.remaining_spend_limit_per_timeframe = ctx
        .accounts
        .vault
        .remaining_spend_limit_per_timeframe
        .checked_sub(amount_usdc_base_units)
        .ok_or(QuartzError::InsufficientTimeframeSpendLimit)?;

    Ok(())
}
