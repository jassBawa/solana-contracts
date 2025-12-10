use crate::errors::ErrorCode;
use crate::state::BridgeConfig;
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

pub fn emergency_withdraw(ctx: Context<EmergencyWithdraw>) -> Result<()> {
    let config = &ctx.accounts.config;
    let config_key = config.key();
    let bump = ctx.accounts.config.vault_authority_bump;

    require!(config.paused, ErrorCode::NotPaused);
    require!(
        config.admin == ctx.accounts.admin.key(),
        ErrorCode::UnauthorizedAdmin
    );

    let vault_seeds: &[&[u8]] = &[b"vault", config_key.as_ref(), &[bump]];
    let signer_seeds = &[vault_seeds];

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            authority: ctx.accounts.vault_authority.to_account_info(),
            from: ctx.accounts.token_vault.to_account_info(),
            to: ctx.accounts.admin_token_account.to_account_info(),
        },
    );

    transfer(
        cpi_ctx.with_signer(signer_seeds),
        ctx.accounts.token_vault.amount,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct EmergencyWithdraw<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds=[b"bridge", config.token_mint.as_ref()],
        bump,
        constraint = admin.key() == config.admin @ErrorCode::UnauthorizedAdmin
    )]
    pub config: Account<'info, BridgeConfig>,

    /// CHECK: Vault autority needed to sign tx
    #[account(
        seeds=[b"vault", config.key().as_ref()],
        bump = config.vault_authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = token_vault.mint == config.token_mint,
        constraint = token_vault.owner == vault_authority.key()
    )]
    pub token_vault: Account<'info, TokenAccount>,

    // During emergencies:
    // Simpler == safer
    // Fewer CPIs
    // Fewer programs involved
    // thats why no init
    #[account(
        mut,
        constraint = admin_token_account.mint == config.token_mint,
        constraint = admin_token_account.owner == admin.key()
    )]
    pub admin_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}
