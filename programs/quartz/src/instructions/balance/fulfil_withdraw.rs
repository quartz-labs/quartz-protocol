use crate::{
    check,
    config::{QuartzError, PUBKEY_SIZE, U64_SIZE, WSOL_MINT},
    state::{Vault, WithdrawOrder},
    utils::{close_time_lock, get_drift_market, validate_time_lock},
};
use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::TokenAccount,
    token_interface::{
        self, close_account, transfer_checked, CloseAccount, Mint,
        TokenAccount as TokenInterfaceAccount, TokenInterface, TransferChecked,
    },
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
use solana_program::{program::invoke_signed, system_instruction};

#[derive(Accounts)]
pub struct FulfilWithdraw<'info> {
    #[account(mut)]
    pub withdraw_order: Box<Account<'info, WithdrawOrder>>,

    /// CHECK: Checked in handler
    #[account(mut)]
    pub time_lock_rent_payer: UncheckedAccount<'info>,

    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: Safe once address is correct
    #[account(
        mut,
        seeds = [b"withdraw_mule".as_ref(), owner.key().as_ref()],
        bump
    )]
    pub mule: UncheckedAccount<'info>,

    /// CHECK: Any account, once it has a vault and matches the order
    #[account(
        mut,
        constraint = owner.key().eq(&withdraw_order.time_lock.owner)
    )]
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = spl_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program
    )]
    pub owner_spl: Option<Box<InterfaceAccount<'info, TokenInterfaceAccount>>>,

    pub spl_mint: Box<InterfaceAccount<'info, Mint>>,

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

    /// CHECK: Safe once seeds are checked
    #[account(
        mut,
        seeds = [b"rent_float".as_ref()],
        bump
    )]
    pub rent_float: UncheckedAccount<'info>,
}

pub fn fulfil_withdraw_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, FulfilWithdraw<'info>>,
) -> Result<()> {
    let (amount_base_units, drift_market_index, reduce_only) = get_order_data(&ctx)?;

    // Validate market index and mint
    let drift_market = get_drift_market(drift_market_index)?;
    check!(
        &ctx.accounts.spl_mint.key().eq(&drift_market.mint),
        QuartzError::InvalidMint
    );

    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let vault_seeds = &[b"vault", owner.as_ref(), &[vault_bump]];
    let vault_signer = &[&vault_seeds[..]];

    let rent_float_bump = ctx.bumps.rent_float;
    let rent_float_seeds = &[b"rent_float".as_ref(), &[rent_float_bump]];
    let rent_float_signer = &[&rent_float_seeds[..]];

    init_ata(&ctx, vault_signer, rent_float_signer)?;

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
        vault_signer,
    );
    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    drift_withdraw(cpi_ctx, drift_market_index, amount_base_units, reduce_only)?;

    // Get true amount withdrawn in case reduce_only prevented full withdraw
    let true_amount_withdrawn = get_ata_balance(&ctx.accounts.mule)?;

    if ctx.accounts.spl_mint.key().eq(&WSOL_MINT) {
        transfer_lamports(&ctx, vault_signer, rent_float_signer, true_amount_withdrawn)?;
    } else {
        transfer_spl(&ctx, vault_signer, true_amount_withdrawn)?;
    }

    Ok(())
}

fn get_order_data(ctx: &Context<FulfilWithdraw>) -> Result<(u64, u16, bool)> {
    validate_time_lock(
        &ctx.accounts.owner.key(),
        &ctx.accounts.withdraw_order.time_lock,
    )?;

    let amount_base_units = ctx.accounts.withdraw_order.amount_base_units;
    let drift_market_index = ctx.accounts.withdraw_order.drift_market_index;
    let reduce_only = ctx.accounts.withdraw_order.reduce_only;

    close_time_lock(
        &ctx.accounts.withdraw_order,
        &ctx.accounts.time_lock_rent_payer.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
    )?;

    Ok((amount_base_units, drift_market_index, reduce_only))
}

fn init_ata(
    ctx: &Context<FulfilWithdraw>,
    vault_signer: &[&[&[u8]]],
    rent_float_signer: &[&[&[u8]]],
) -> Result<()> {
    check!(
        ctx.accounts.mule.data_is_empty(),
        QuartzError::AccountAlreadyInitialized
    );

    let space = TokenAccount::LEN;
    let rent = Rent::get()?;
    let lamports_required = rent.minimum_balance(space);

    // Create account
    create_account(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            CreateAccount {
                from: ctx.accounts.rent_float.to_account_info(),
                to: ctx.accounts.mule.to_account_info(),
            },
            rent_float_signer,
        ),
        lamports_required,
        space as u64,
        &ctx.accounts.token_program.key(),
    )?;

    // Init ATA
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token_interface::InitializeAccount3 {
            account: ctx.accounts.mule.to_account_info(),
            mint: ctx.accounts.spl_mint.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        vault_signer,
    );
    token_interface::initialize_account3(cpi_ctx)?;

    Ok(())
}

fn get_ata_balance(ata: &AccountInfo) -> Result<u64> {
    let data: &[u8] = &ata.try_borrow_data()?;
    let amount_start_index = PUBKEY_SIZE + PUBKEY_SIZE;
    let amount_bytes = &data[amount_start_index..amount_start_index + U64_SIZE];
    let amount = u64::from_le_bytes(
        amount_bytes
            .try_into()
            .expect("Failed to convert amount bytes to u64"),
    );
    Ok(amount)
}

fn transfer_lamports(
    ctx: &Context<FulfilWithdraw>,
    vault_signer: &[&[&[u8]]],
    rent_float_signer: &[&[&[u8]]],
    true_amount_withdrawn: u64,
) -> Result<()> {
    // Close wSOL mule, unwrapping all SOL to rent_float
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.mule.to_account_info(),
            destination: ctx.accounts.rent_float.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        vault_signer,
    );
    close_account(cpi_ctx_close)?;

    // Send true_amount_withdrawn to the owner, leaving just the ATA rent remaining
    invoke_signed(
        &system_instruction::transfer(
            &ctx.accounts.rent_float.key(),
            &ctx.accounts.owner.key(),
            true_amount_withdrawn,
        ),
        &[
            ctx.accounts.rent_float.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        rent_float_signer,
    )?;

    Ok(())
}

fn transfer_spl(
    ctx: &Context<FulfilWithdraw>,
    vault_signer: &[&[&[u8]]],
    true_amount_withdrawn: u64,
) -> Result<()> {
    let owner_spl = match ctx.accounts.owner_spl.as_ref() {
        Some(owner_spl) => owner_spl,
        None => return Err(QuartzError::InvalidOwnerSplWSOL.into()), // owner_spl is only optional for wSOL
    };

    // Transfer all tokens from mule to owner_spl
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.mule.to_account_info(),
                to: owner_spl.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.spl_mint.to_account_info(),
            },
            vault_signer,
        ),
        true_amount_withdrawn,
        ctx.accounts.spl_mint.decimals,
    )?;

    // Close mule
    let cpi_ctx_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.mule.to_account_info(),
            destination: ctx.accounts.rent_float.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        },
        vault_signer,
    );
    close_account(cpi_ctx_close)?;

    Ok(())
}
