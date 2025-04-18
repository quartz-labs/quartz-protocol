use crate::{
    check,
    config::{QuartzError, ANCHOR_DISCRIMINATOR},
    state::{CollateralRepayLedger, Vault},
};
use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        sysvar::instructions::{self, load_current_index_checked, load_instruction_at_checked},
    },
    Discriminator,
};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use solana_program::instruction::get_stack_height;

#[derive(Accounts)]
pub struct StartCollateralRepay<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = mint_deposit,
        associated_token::authority = caller,
        associated_token::token_program = token_program_deposit
    )]
    pub caller_deposit_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_withdraw,
        associated_token::authority = caller,
        associated_token::token_program = token_program_withdraw
    )]
    pub caller_withdraw_spl: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Can be any account, once it has a Vault
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub mint_deposit: Box<InterfaceAccount<'info, Mint>>,

    pub mint_withdraw: Box<InterfaceAccount<'info, Mint>>,

    pub token_program_deposit: Interface<'info, TokenInterface>,

    pub token_program_withdraw: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,

    /// CHECK: Account is safe once address is correct
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [b"collateral_repay_ledger".as_ref(), owner.key().as_ref()],
        bump,
        payer = caller,
        space = CollateralRepayLedger::INIT_SPACE
    )]
    pub ledger: Box<Account<'info, CollateralRepayLedger>>,
}

pub fn start_collateral_repay_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, StartCollateralRepay<'info>>,
) -> Result<()> {
    let index: usize =
        load_current_index_checked(&ctx.accounts.instructions.to_account_info())?.into();
    let current_instruction =
        load_instruction_at_checked(index, &ctx.accounts.instructions.to_account_info())?;
    let swap_instruction =
        load_instruction_at_checked(index + 1, &ctx.accounts.instructions.to_account_info())?;
    let deposit_instruction =
        load_instruction_at_checked(index + 2, &ctx.accounts.instructions.to_account_info())?;
    let withdraw_instruction =
        load_instruction_at_checked(index + 3, &ctx.accounts.instructions.to_account_info())?;

    validate_instruction_order(
        &current_instruction,
        &swap_instruction,
        &deposit_instruction,
        &withdraw_instruction,
    )?;

    validate_user_accounts_context(&deposit_instruction, &withdraw_instruction)?;

    validate_drift_markets(&deposit_instruction, &withdraw_instruction)?;

    validate_spl_context(&ctx, &deposit_instruction, &withdraw_instruction)?;

    // Log deposit and withdraw starting balances (ensures deposit & withdraw use exact amounts in swap)
    // It's fine if this account already exists, as we're overwriting the values
    let ledger = &mut ctx.accounts.ledger;
    ledger.deposit = ctx.accounts.caller_deposit_spl.amount;
    ledger.withdraw = ctx.accounts.caller_withdraw_spl.amount;

    Ok(())
}

#[inline(never)]
pub fn validate_instruction_order(
    current_instruction: &Instruction,
    swap_instruction: &Instruction,
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction,
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

    // 2nd instruction can be anything, once it's not a Quartz instruction (prevent reentrancy)
    check!(
        !swap_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayCPI
    );

    // 3rd instruction must be deposit_collateral_repay
    check!(
        deposit_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        deposit_instruction.data[..ANCHOR_DISCRIMINATOR]
            .eq(&crate::instruction::DepositCollateralRepay::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    // 4th instruction must be withdraw_collateral_repay
    check!(
        withdraw_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        withdraw_instruction.data[..ANCHOR_DISCRIMINATOR]
            .eq(&crate::instruction::WithdrawCollateralRepay::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    Ok(())
}

#[inline(never)]
fn validate_user_accounts_context(
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction,
) -> Result<()> {
    let deposit_caller = deposit_instruction.accounts[0].pubkey;
    let withdraw_caller = withdraw_instruction.accounts[0].pubkey;
    check!(
        deposit_caller.eq(&withdraw_caller),
        QuartzError::InvalidUserAccounts
    );

    let deposit_owner = deposit_instruction.accounts[2].pubkey;
    let withdraw_owner = withdraw_instruction.accounts[2].pubkey;
    check!(
        deposit_owner.eq(&withdraw_owner),
        QuartzError::InvalidUserAccounts
    );

    // Vault and drift accounts will all be the same if owner is the same

    Ok(())
}

#[inline(never)]
fn validate_drift_markets(
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction,
) -> Result<()> {
    let deposit_market_index = u16::from_le_bytes(
        deposit_instruction.data[8..10]
            .try_into()
            .map_err(|_| QuartzError::FailedToDeserializeMarketIndex)?,
    );
    let withdraw_market_index = u16::from_le_bytes(
        withdraw_instruction.data[8..10]
            .try_into()
            .map_err(|_| QuartzError::FailedToDeserializeMarketIndex)?,
    );

    check!(
        !deposit_market_index.eq(&withdraw_market_index),
        QuartzError::IdenticalCollateralRepayMarkets
    );

    // Deposit & withdraw instructions themselves check that the mints match the index

    Ok(())
}

#[inline(never)]
fn validate_spl_context<'info>(
    ctx: &Context<'_, '_, 'info, 'info, StartCollateralRepay<'info>>,
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction,
) -> Result<()> {
    // Validate mints
    let deposit_mint = deposit_instruction.accounts[5].pubkey;
    check!(
        ctx.accounts.mint_deposit.key().eq(&deposit_mint),
        QuartzError::InvalidMint
    );

    let withdraw_mint = withdraw_instruction.accounts[5].pubkey;
    check!(
        ctx.accounts.mint_withdraw.key().eq(&withdraw_mint),
        QuartzError::InvalidMint
    );

    // Validate ATAs
    let deposit_spl_account = deposit_instruction.accounts[1].pubkey;
    check!(
        ctx.accounts
            .caller_deposit_spl
            .key()
            .eq(&deposit_spl_account),
        QuartzError::InvalidSourceTokenAccount
    );

    let withdraw_spl_account = withdraw_instruction.accounts[1].pubkey;
    check!(
        ctx.accounts
            .caller_withdraw_spl
            .key()
            .eq(&withdraw_spl_account),
        QuartzError::InvalidSourceTokenAccount
    );

    Ok(())
}
