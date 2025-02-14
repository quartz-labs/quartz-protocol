use anchor_lang::prelude::*;
use drift::{
    program::Drift, state::{
        state::State as DriftState, 
        user::{User as DriftUser, UserStats as DriftUserStats}
    }
};
use solana_program::{instruction::Instruction, program::invoke_signed, system_instruction};
use crate::{config::{DRIFT_DELETE_USER_DISCRIMINATOR, INIT_ACCOUNT_RENT_FEE}, state::Vault};

#[derive(Accounts)]
pub struct CloseUser<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner,
        close = init_rent_payer,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"init_rent_payer"],
        bump
    )]
    pub init_rent_payer: UncheckedAccount<'info>,

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

    pub drift_program: Program<'info, Drift>,

    pub system_program: Program<'info, System>
}

pub fn close_user_handler(ctx: Context<CloseUser>) -> Result<()> {
    let vault_bump = ctx.accounts.vault.bump;
    let owner = ctx.accounts.owner.key();
    let seeds = &[
        b"vault",
        owner.as_ref(),
        &[vault_bump]
    ];
    let signer_seeds = &[&seeds[..]];

    // let delete_user_cpi_context = CpiContext::new_with_signer(
    //     ctx.accounts.drift_program.to_account_info(),
    //     DeleteUser {
    //         user: ctx.accounts.drift_user.to_account_info(),
    //         user_stats: ctx.accounts.drift_user_stats.to_account_info(),
    //         state: ctx.accounts.drift_state.to_account_info(),
    //         authority: ctx.accounts.vault.to_account_info()
    //     },
    //     signer_seeds
    // );

    // delete_user(delete_user_cpi_context)?;

    // Drift's CPI crate will cause CloseAccount to fail as the authority account is incorrectly marked as immutable
    // Instead, manually create instruction data using Drift's instruction descriminator to get around the accounts struct
    let ix = Instruction {
        program_id: ctx.accounts.drift_program.key(),
        accounts: vec![
            AccountMeta::new(ctx.accounts.drift_user.key(), false),
            AccountMeta::new(ctx.accounts.drift_user_stats.key(), false),
            AccountMeta::new(ctx.accounts.drift_state.key(), false),
            AccountMeta::new(ctx.accounts.vault.key(), true),
        ],
        data: DRIFT_DELETE_USER_DISCRIMINATOR.to_vec(),
    };

    invoke_signed(
        &ix,
        &[
            ctx.accounts.drift_user.to_account_info(),
            ctx.accounts.drift_user_stats.to_account_info(),
            ctx.accounts.drift_state.to_account_info(),
            ctx.accounts.vault.to_account_info(),
        ],
        signer_seeds,
    )?;

    // Repay user the init rent fee
    invoke_signed(
        &system_instruction::transfer(
            ctx.accounts.init_rent_payer.key, 
            ctx.accounts.owner.key, 
            INIT_ACCOUNT_RENT_FEE
        ),
        &[
            ctx.accounts.init_rent_payer.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds
    )?;

    Ok(())
}