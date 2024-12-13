use anchor_lang::prelude::*;
use drift::{
    program::Drift, state::{
        state::State as DriftState, 
        user::{User as DriftUser, UserStats as DriftUserStats}
    }
};
use solana_program::{instruction::Instruction, program::invoke_signed};
use crate::state::Vault;

#[derive(Accounts)]
pub struct CloseDriftAccount<'info> {
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,

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

    pub drift_program: Program<'info, Drift>
}

pub fn close_drift_account_handler(
    ctx: Context<CloseDriftAccount>
) -> Result<()> {    
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
        data: vec![186, 85, 17, 249, 219, 231, 98, 251],
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

    Ok(())
}