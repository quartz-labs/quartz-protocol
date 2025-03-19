use anchor_lang::{
    prelude::*, solana_program::sysvar::instructions::{
        self,
        load_current_index_checked, 
        load_instruction_at_checked
    }, Discriminator
};
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{
        Mint, 
        TokenAccount, 
        TokenInterface, 
        TransferChecked,
        transfer_checked
    }
};
use drift::{
    program::Drift,
    cpi::withdraw as drift_withdraw, 
    cpi::accounts::Withdraw as DriftWithdraw,
    state::{
        state::State as DriftState, 
        user::{User as DriftUser, UserStats as DriftUserStats}
    }
};
use solana_program::instruction::Instruction;
use crate::{
    check, config::{QuartzError, SPEND_CALLER, SPEND_FEE_BPS, SPEND_FEE_DESTINATION, USDC_MARKET_INDEX}, state::Vault, utils::get_drift_market
};

#[event_cpi]
#[derive(Accounts)]
pub struct StartSpend<'info> {
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

    /// CHECK: Address is checked, and owner ensures it's a token account
    #[account(
        mut,
        constraint = spend_fee_destination.key().eq(&SPEND_FEE_DESTINATION),
        constraint = spend_fee_destination.owner.eq(&token_program.key())
    )]
    pub spend_fee_destination: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [b"spend_mule".as_ref(), owner.key().as_ref()],
        bump,
        payer = spend_caller,
        token::mint = usdc_mint,
        token::authority = vault
    )]
    pub mule: Box<InterfaceAccount<'info, TokenAccount>>,

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

    /// CHECK: Account is safe once address is correct
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>
}

pub fn start_spend_handler<'info>(
    mut ctx: Context<'_, '_, '_, 'info, StartSpend<'info>>, 
    amount_usdc_base_units: u64,
    spend_fee: bool
) -> Result<()> {
    let index: usize = load_current_index_checked(
        &ctx.accounts.instructions.to_account_info()
    )?.into();
    let complete_instruction = load_instruction_at_checked(
        index + 1, 
        &ctx.accounts.instructions.to_account_info()
    )?;
    validate_complete_spend_ix(&ctx, &complete_instruction)?;

    // Validate market index and mint
    let drift_market = get_drift_market(USDC_MARKET_INDEX)?;
    check!(
        &ctx.accounts.usdc_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    process_spend_limits(&mut ctx, amount_usdc_base_units)?;
    
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    // Use Drift Withdraw CPI to transfer USDC to spend mule
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
        signer_seeds
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    drift_withdraw(cpi_ctx, USDC_MARKET_INDEX, amount_usdc_base_units, false)?;

    // If taking a fee, transfer fee from mule to spend caller
    if spend_fee {
        let fee_amount = (amount_usdc_base_units * SPEND_FEE_BPS) / 10_000;
        
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(), 
                TransferChecked { 
                    from: ctx.accounts.mule.to_account_info(), 
                    to: ctx.accounts.spend_fee_destination.to_account_info(), 
                    authority: ctx.accounts.spend_caller.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                }
            ),
            fee_amount,
            ctx.accounts.usdc_mint.decimals
        )?;
    }

    Ok(())
}

pub fn validate_complete_spend_ix<'info>(
    ctx: &Context<'_, '_, '_, 'info, StartSpend<'info>>,
    complete_spend: &Instruction
) -> Result<()> {
    check!(
        complete_spend.program_id.eq(&crate::id()),
        QuartzError::IllegalSpendInstructions
    );

    check!(
        complete_spend.data[..8]
            .eq(&crate::instruction::CompleteSpend::DISCRIMINATOR),
        QuartzError::IllegalSpendInstructions
    );

    // Validate state
    let complete_vault = complete_spend.accounts[0].pubkey;
    check!(
        complete_vault.eq(&ctx.accounts.vault.key()),
        QuartzError::InvalidUserAccounts
    );

    let complete_owner = complete_spend.accounts[1].pubkey;
    check!(
        complete_owner.eq(&ctx.accounts.owner.key()),
        QuartzError::InvalidUserAccounts
    );

    Ok(())
}

fn process_spend_limits<'info>(
    ctx: &mut Context<'_, '_, '_, 'info, StartSpend<'info>>,
    amount_usdc_base_units: u64
) -> Result<()> {
    let current_timestamp_raw = Clock::get()?.unix_timestamp;
    check!(
        current_timestamp_raw > 0,
        QuartzError::InvalidTimestamp
    );
    let current_timestamp = current_timestamp_raw as u64;

    check!(
        ctx.accounts.vault.spend_limit_per_transaction >= amount_usdc_base_units,
        QuartzError::InsufficientTransactionSpendLimit
    );

    check!(
        ctx.accounts.vault.timeframe_in_seconds > 0,
        QuartzError::InsufficientTimeframeSpendLimit
    );

    // If the timeframe has elapsed, incrememt it and reset spend limit
    if current_timestamp >= ctx.accounts.vault.next_timeframe_reset_timestamp {
        let overflow = current_timestamp - ctx.accounts.vault.next_timeframe_reset_timestamp;
        let overflow_in_timeframes = overflow / ctx.accounts.vault.timeframe_in_seconds;
        let seconds_to_add = (overflow_in_timeframes + 1)
            .checked_mul(ctx.accounts.vault.timeframe_in_seconds)
            .ok_or(QuartzError::MathOverflow)?;

        ctx.accounts.vault.next_timeframe_reset_timestamp = ctx.accounts.vault.next_timeframe_reset_timestamp
            .checked_add(seconds_to_add)
            .ok_or(QuartzError::MathOverflow)?;
        ctx.accounts.vault.remaining_spend_limit_per_timeframe = ctx.accounts.vault.spend_limit_per_timeframe;
    }

    check!(
        ctx.accounts.vault.remaining_spend_limit_per_timeframe >= amount_usdc_base_units,
        QuartzError::InsufficientTimeframeSpendLimit
    );

    // Adjust remaining spend limit
    ctx.accounts.vault.remaining_spend_limit_per_timeframe = ctx.accounts.vault.remaining_spend_limit_per_timeframe
        .checked_sub(amount_usdc_base_units)
        .ok_or(QuartzError::InsufficientTimeframeSpendLimit)?;

    Ok(())
}