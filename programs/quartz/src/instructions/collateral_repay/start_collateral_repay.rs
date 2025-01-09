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
    associated_token::AssociatedToken,
    token_interface::{
        TransferChecked,
        transfer_checked,
        TokenInterface, 
        TokenAccount, 
        Mint,
        CloseAccount,
        close_account
    }
};
use drift::{
    cpi::{
        accounts::Deposit as DriftDeposit,
        deposit as drift_deposit
    }, 
    state::{
        state::State as DriftState, 
        user::User as DriftUser
    },
    program::Drift
};
use crate::{
    check, 
    config::QuartzError, 
    load_mut, 
    state::Vault, 
    utils::{
        get_drift_margin_calculation, 
        get_drift_market, 
        get_quartz_account_health
    }
};

#[derive(Accounts)]
pub struct StartCollateralRepay<'info> {
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

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>,

    /// CHECK: Account is safe once address is correct
    #[account(address = instructions::ID)]
    instructions: UncheckedAccount<'info>,
}

#[inline(never)]
fn validate_instruction_order<'info>(
    end_instruction: &Instruction
) -> Result<()> {
    // Check the next instruction is end_collateral_repay
    check!(
        end_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        end_instruction.data[..8]
            .eq(&crate::instruction::EndCollateralRepay::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    Ok(())
}

fn validate_drift_markets<'info>(
    deposit_market_index: u16,
    spl_mint: &Pubkey,
    end_instruction: &Instruction
) -> Result<(u16, u16)> {
    let drift_market = get_drift_market(deposit_market_index)?;
    check!(
        spl_mint.eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    let withdraw_drift_market_index = u16::from_le_bytes(end_instruction.data[16..18].try_into().unwrap());
    check!(
        !drift_market.market_index.eq(&withdraw_drift_market_index),
        QuartzError::IdenticalCollateralRepayMarkets
    );

    Ok((deposit_market_index, withdraw_drift_market_index))
}

#[inline(never)]
fn validate_account_health<'info>(
    ctx: &Context<'_, '_, 'info, 'info, StartCollateralRepay<'info>>,
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
        quartz_account_health == 0,
        QuartzError::NotReachedCollateralRepayThreshold
    );

    Ok(())
}

pub fn start_collateral_repay_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, StartCollateralRepay<'info>>,
    amount_deposit_base_units: u64,
    drift_market_index: u16
) -> Result<()> {
    let index: usize = load_current_index_checked(&ctx.accounts.instructions.to_account_info())?.into();
    let end_instruction = load_instruction_at_checked(index + 1, &ctx.accounts.instructions.to_account_info())?;

    validate_instruction_order(&end_instruction)?;

    let (
        deposit_market_index, 
        withdraw_market_index
    ) = validate_drift_markets(drift_market_index, &ctx.accounts.spl_mint.key(), &end_instruction)?;

    // Validate account health if the owner isn't the caller
    if !ctx.accounts.owner.key().eq(&ctx.accounts.caller.key()) {
        validate_account_health(&ctx, deposit_market_index, withdraw_market_index)?;
    }

    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    // Transfer tokens from callers's ATA to vault's ATA
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            TransferChecked { 
                from: ctx.accounts.caller_spl.to_account_info(), 
                to: ctx.accounts.vault_spl.to_account_info(), 
                authority: ctx.accounts.caller.to_account_info(),
                mint: ctx.accounts.spl_mint.to_account_info(),
            }
        ),
        amount_deposit_base_units,
        ctx.accounts.spl_mint.decimals
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
            user_token_account: ctx.accounts.vault_spl.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    // reduce_only = true means that the caller can not deposit more than the user's borrowed position / create a collateral position
    drift_deposit(cpi_ctx, drift_market_index, amount_deposit_base_units, true)?;

    // Return any remaining balance (in case return_only prevented full deposit)
    let remaining_balance = ctx.accounts.vault_spl.amount;
    if remaining_balance > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                TransferChecked { 
                    from: ctx.accounts.vault_spl.to_account_info(), 
                    to: ctx.accounts.caller_spl.to_account_info(), 
                    authority: ctx.accounts.vault.to_account_info(),
                    mint: ctx.accounts.spl_mint.to_account_info(),
                }, 
                signer_seeds
            ),
            remaining_balance,
            ctx.accounts.spl_mint.decimals
        )?;
    }

    // Close vault's ATA
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault_spl.to_account_info(),
            destination: ctx.accounts.caller.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        signer_seeds
    );
    close_account(cpi_ctx_close)?;

    Ok(())
}