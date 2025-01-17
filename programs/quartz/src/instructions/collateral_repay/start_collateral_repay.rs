use anchor_lang::{
    prelude::*, solana_program::{
        instruction::Instruction, 
        sysvar::instructions::{
            self,
            load_current_index_checked, 
            load_instruction_at_checked
        }
    }, 
    Discriminator
};
use anchor_spl::token_interface::{
    TokenInterface, 
    TokenAccount, 
    Mint
};
use crate::{
    check, 
    config::{QuartzError, JUPITER_ID, JUPITER_SWAP_DISCRIMINATORS},
    state::{CollateralRepayLedger, Vault}
};

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
        bump = vault.bump,
        has_one = owner
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
    let index: usize = load_current_index_checked(
        &ctx.accounts.instructions.to_account_info()
    )?.into();
    let swap_instruction = load_instruction_at_checked(
        index + 1, 
        &ctx.accounts.instructions.to_account_info()
    )?;
    let deposit_instruction = load_instruction_at_checked(
        index + 2, 
        &ctx.accounts.instructions.to_account_info()
    )?;
    let withdraw_instruction = load_instruction_at_checked(
        index + 3, 
        &ctx.accounts.instructions.to_account_info()
    )?;
    
    validate_instruction_order(&swap_instruction, &deposit_instruction, &withdraw_instruction)?;

    validate_user_accounts_context(&deposit_instruction, &withdraw_instruction)?;

    validate_drift_markets(&deposit_instruction, &withdraw_instruction)?;

    validate_swap_context(&swap_instruction, &deposit_instruction, &withdraw_instruction)?;

    // Log deposit and withdraw starting balances
    let ledger = &mut ctx.accounts.ledger;
    ledger.deposit = ctx.accounts.caller_deposit_spl.amount;
    ledger.withdraw = ctx.accounts.caller_withdraw_spl.amount;

    Ok(())
}

#[inline(never)]
pub fn validate_instruction_order<'info>(
    swap_instruction: &Instruction,
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction,
) -> Result<()> {
    // This is the 1st ix

    // Check Jupiter swap (2nd ix)
    check!(
        swap_instruction.program_id.eq(&JUPITER_ID),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        JUPITER_SWAP_DISCRIMINATORS.iter().any(|discriminator| 
            swap_instruction.data[..8].eq(discriminator)
        ),
        QuartzError::IllegalCollateralRepayInstructions
    );

    // Check deposit_collateral_repay (3rd ix)
    check!(
        deposit_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        deposit_instruction.data[..8]
            .eq(&crate::instruction::DepositCollateralRepay::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    // Check withdraw_collateral_repay (4th ix)
    check!(
        withdraw_instruction.program_id.eq(&crate::id()),
        QuartzError::IllegalCollateralRepayInstructions
    );

    check!(
        withdraw_instruction.data[..8]
            .eq(&crate::instruction::WithdrawCollateralRepay::DISCRIMINATOR),
        QuartzError::IllegalCollateralRepayInstructions
    );

    Ok(())
}

#[inline(never)]
fn validate_user_accounts_context<'info>(
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction
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

    let deposit_vault = deposit_instruction.accounts[3].pubkey;
    let withdraw_vault = withdraw_instruction.accounts[3].pubkey;
    check!(
        deposit_vault.eq(&withdraw_vault),
        QuartzError::InvalidUserAccounts
    );

    let deposit_drift_user = deposit_instruction.accounts[6].pubkey;
    let withdraw_drift_user = withdraw_instruction.accounts[6].pubkey;
    check!(
        deposit_drift_user.eq(&withdraw_drift_user),
        QuartzError::InvalidUserAccounts
    );

    let deposit_drift_user_stats = deposit_instruction.accounts[7].pubkey;
    let withdraw_drift_user_stats = withdraw_instruction.accounts[7].pubkey;
    check!(
        deposit_drift_user_stats.eq(&withdraw_drift_user_stats),
        QuartzError::InvalidUserAccounts
    );

    Ok(())
}

#[inline(never)]
fn validate_drift_markets<'info>(
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction
) -> Result<()> {
    let deposit_market_index = u16::from_le_bytes(deposit_instruction.data[16..18].try_into().unwrap());
    let withdraw_market_index = u16::from_le_bytes(withdraw_instruction.data[16..18].try_into().unwrap());
    check!(
        !deposit_market_index.eq(&withdraw_market_index),
        QuartzError::IdenticalCollateralRepayMarkets
    );

    Ok(())
}

#[inline(never)]
fn validate_swap_context<'info>(
    swap_instruction: &Instruction,
    deposit_instruction: &Instruction,
    withdraw_instruction: &Instruction
) -> Result<()> {
    // Validate mints
    let swap_out_mint = swap_instruction.accounts[6].pubkey;
    let deposit_mint = deposit_instruction.accounts[5].pubkey;
    check!(
        swap_out_mint.eq(&deposit_mint),
        QuartzError::InvalidMint
    );

    let swap_in_mint = swap_instruction.accounts[5].pubkey;
    let withdraw_mint = withdraw_instruction.accounts[5].pubkey;
    check!(
        swap_in_mint.eq(&withdraw_mint),
        QuartzError::InvalidMint
    );

    // Validate ATAs
    let swap_out_account = swap_instruction.accounts[3].pubkey;
    let deposit_spl_account = deposit_instruction.accounts[1].pubkey;
    check!(
        swap_out_account.eq(&deposit_spl_account),
        QuartzError::InvalidSourceTokenAccount
    );

    let swap_in_account = swap_instruction.accounts[2].pubkey;
    let withdraw_spl_account = withdraw_instruction.accounts[1].pubkey;
    check!(
        swap_in_account.eq(&withdraw_spl_account),
        QuartzError::InvalidSourceTokenAccount
    );

    Ok(())
}
