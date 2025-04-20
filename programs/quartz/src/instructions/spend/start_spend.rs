use crate::{
    check,
    config::{
        QuartzError, ANCHOR_DISCRIMINATOR, SPEND_CALLER, SPEND_FEE_BPS, SPEND_FEE_DESTINATION,
        USDC_MARKET_INDEX,
    },
    state::Vault,
    utils::{get_drift_market, validate_ata},
};
use anchor_lang::{
    prelude::*,
    solana_program::sysvar::instructions::{
        self, load_current_index_checked, load_instruction_at_checked,
    },
    Discriminator,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use drift::{
    cpi::accounts::Withdraw as DriftWithdraw, cpi::withdraw as drift_withdraw, program::Drift,
};
use solana_program::instruction::{get_stack_height, Instruction};

#[event_cpi]
#[derive(Accounts)]
pub struct StartSpend<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: Can be any account, once it has a Vault
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = spend_caller.key().eq(&SPEND_CALLER) @ QuartzError::InvalidSpendCaller
    )]
    pub spend_caller: Signer<'info>,

    /// CHECK: Safe once address is correct
    #[account(
        mut,
        constraint = spend_fee_destination.key().eq(&SPEND_FEE_DESTINATION) @ QuartzError::InvalidSpendFeeDestination
    )]
    pub spend_fee_destination: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        seeds = [b"spend_mule".as_ref(), owner.key().as_ref()],
        bump,
        payer = spend_caller,
        token::mint = usdc_mint,
        token::authority = vault
    )]
    pub mule: Box<InterfaceAccount<'info, TokenAccount>>,

    pub usdc_mint: Box<InterfaceAccount<'info, Mint>>,

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

    /// CHECK: This account is passed through to the Drift CPI, which performs the security checks
    pub drift_signer: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    /// CHECK: Account is safe once address is correct
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Safe once seeds are correct, deposit address is the pubkey anyone can send tokens to for deposits
    #[account(
        seeds = [b"deposit_address".as_ref(), vault.key().as_ref()],
        bump
    )]
    pub deposit_address: UncheckedAccount<'info>,

    /// CHECK: Checked in handler as the account doesn't need to exist
    #[account(mut)]
    pub deposit_address_usdc: UncheckedAccount<'info>,
}

/// First spend instruction (split due to stack size limits), withdraws from vault and updates spend limits
pub fn start_spend_handler<'info>(
    mut ctx: Context<'_, '_, '_, 'info, StartSpend<'info>>,
    amount_usdc_base_units: u64,
    spend_fee: bool,
) -> Result<()> {
    let index: usize =
        load_current_index_checked(&ctx.accounts.instructions.to_account_info())?.into();
    let current_instruction =
        load_instruction_at_checked(index, &ctx.accounts.instructions.to_account_info())?;
    let complete_instruction =
        load_instruction_at_checked(index + 1, &ctx.accounts.instructions.to_account_info())?;
    validate_complete_spend_ix(&ctx, &current_instruction, &complete_instruction)?;

    // Manually check mint in handler to avoid Anchor stack overflow
    let drift_market = get_drift_market(USDC_MARKET_INDEX)?;
    check!(
        &ctx.accounts.usdc_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    // Fee destination must be the token account itself, not the authority
    check!(
        &ctx.accounts
            .spend_fee_destination
            .owner
            .eq(&ctx.accounts.token_program.key()),
        QuartzError::InvalidSpendFeeDestination
    );

    process_spend_limits(&mut ctx, amount_usdc_base_units)?;

    let deposit_address_usdc = validate_ata(
        &ctx.accounts.deposit_address_usdc.to_account_info(),
        &ctx.accounts.deposit_address.to_account_info(),
        &ctx.accounts.usdc_mint.to_account_info(),
        &ctx.accounts.token_program,
    )?;

    // First withdraw any idle funds from deposit address
    let idle_funds = match deposit_address_usdc {
        Some(account) => account.amount.min(amount_usdc_base_units),
        None => 0, // If ATA doesn't exist, there are no idle funds
    };

    if idle_funds > 0 {
        let deposit_address_bump = ctx.bumps.deposit_address;
        let vault = ctx.accounts.vault.key();
        let seeds_deposit_address = &[b"deposit_address", vault.as_ref(), &[deposit_address_bump]];
        let deposit_address_signer = &[&seeds_deposit_address[..]];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.deposit_address_usdc.to_account_info(),
                    to: ctx.accounts.mule.to_account_info(),
                    authority: ctx.accounts.deposit_address.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                },
                deposit_address_signer,
            ),
            idle_funds,
            ctx.accounts.usdc_mint.decimals,
        )?;
    };

    // Withdraw required funds remaining from Drift
    let required_funds_remaining = amount_usdc_base_units.saturating_sub(idle_funds);
    if required_funds_remaining > 0 {
        let vault_bump = ctx.accounts.vault.bump;
        let owner = ctx.accounts.owner.key();
        let seeds_vault = &[b"vault", owner.as_ref(), &[vault_bump]];
        let vault_signer = &[&seeds_vault[..]];

        let vault_lamports_before_cpi = ctx.accounts.vault.to_account_info().lamports();

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

        // reduce_only = false to allow for collateral position becoming a loan
        drift_withdraw(cpi_ctx, USDC_MARKET_INDEX, required_funds_remaining, false)?;

        // Reload vault data to ensure it hasn't been drained by the Drift CPI
        ctx.accounts.vault.reload()?;
        let vault_lamports_after_cpi = ctx.accounts.vault.to_account_info().lamports();
        check!(
            vault_lamports_after_cpi >= vault_lamports_before_cpi,
            QuartzError::IllegalVaultCPIModification
        );
    }

    // If taking a fee, transfer cut of amount from mule to spend caller
    if spend_fee {
        // Sanity check on spend fee
        const MAX_SPEND_FEE_BPS: u64 = 500;
        check!(
            SPEND_FEE_BPS <= MAX_SPEND_FEE_BPS,
            QuartzError::InvalidSpendFeeBPS
        );

        let fee_amount = amount_usdc_base_units
            .checked_mul(SPEND_FEE_BPS)
            .ok_or(QuartzError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(QuartzError::MathOverflow)?;

        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.mule.to_account_info(),
                    to: ctx.accounts.spend_fee_destination.to_account_info(),
                    authority: ctx.accounts.spend_caller.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                },
            ),
            fee_amount,
            ctx.accounts.usdc_mint.decimals,
        )?;
    }

    Ok(())
}

pub fn validate_complete_spend_ix<'info>(
    ctx: &Context<'_, '_, '_, 'info, StartSpend<'info>>,
    current_instruction: &Instruction,
    complete_spend: &Instruction,
) -> Result<()> {
    // Ensure we're not in a CPI (to validate introspection)
    const TOP_LEVEL_STACK_HEIGHT: usize = 1;
    check!(
        get_stack_height() == TOP_LEVEL_STACK_HEIGHT,
        QuartzError::IllegalCollateralRepayCPI
    );
    check!(
        current_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayCPI
    );

    // Validate instruction
    check!(
        complete_spend.program_id.eq(&crate::id()),
        QuartzError::IllegalSpendInstructions
    );

    check!(
        complete_spend.data[..ANCHOR_DISCRIMINATOR]
            .eq(&crate::instruction::CompleteSpend::DISCRIMINATOR),
        QuartzError::IllegalSpendInstructions
    );

    // Validate state
    let complete_owner = complete_spend.accounts[1].pubkey;
    check!(
        complete_owner.eq(&ctx.accounts.owner.key()),
        QuartzError::InvalidUserAccounts
    );

    // Vault will be the same if owner is the same

    Ok(())
}

fn process_spend_limits<'info>(
    ctx: &mut Context<'_, '_, '_, 'info, StartSpend<'info>>,
    amount_usdc_base_units: u64,
) -> Result<()> {
    let current_timestamp_signed = Clock::get()?.unix_timestamp;
    check!(current_timestamp_signed > 0, QuartzError::InvalidTimestamp);
    let current_timestamp =
        u64::try_from(current_timestamp_signed).map_err(|_| QuartzError::MathOverflow)?;

    // Check transaction spend limit and timeframe
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
    // New reset timestamp = old reset timestamp + the amount of timeframes required to reach a timestamp in the future
    if current_timestamp > ctx.accounts.vault.next_timeframe_reset_timestamp {
        let overflow = current_timestamp
            .checked_sub(ctx.accounts.vault.next_timeframe_reset_timestamp)
            .ok_or(QuartzError::MathOverflow)?;

        // Can't divide by 0 as it's checked earlier
        let overflow_in_timeframes = overflow / ctx.accounts.vault.timeframe_in_seconds;

        let seconds_to_add = overflow_in_timeframes
            .checked_add(1) // Bring the next reset into the future
            .ok_or(QuartzError::MathOverflow)?
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

    // Check remaining spend limit
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
