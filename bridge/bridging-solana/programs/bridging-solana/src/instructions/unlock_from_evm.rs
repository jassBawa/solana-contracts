use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    errors::ErrorCode,
    state::{BridgeConfig, ProcessedMessage},
};

pub fn unlock_from_evm(
    ctx: Context<UnlockFromEvm>,
    _src_chain_id: u64,
    _nonce: u64,
    amount: u64,
) -> Result<()> {
    let config = &ctx.accounts.config;
    let config_key = config.key();
    require!(!config.paused, ErrorCode::BridgePaused);
    require!(
        ctx.accounts.relayer.key() == config.relayer_pubkey,
        ErrorCode::Unauthorized
    );

    require!(
        !ctx.accounts.processed_message.executed,
        ErrorCode::AlreadyProcessed
    );

    ctx.accounts.processed_message.executed = true;

    // transfer
    let vault_seeds: &[&[u8]] = &[
        b"vault",
        config_key.as_ref(),
        &[config.vault_authority_bump],
    ];
    let signer_seeds = &[vault_seeds];

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            authority: ctx.accounts.vault_authority.to_account_info(),
            from: ctx.accounts.token_vault.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
        },
    );

    transfer(cpi_ctx.with_signer(signer_seeds), amount)?;
    Ok(())
}

#[derive(Accounts)]
#[instruction(src_chain_id: u64, nonce: u64)]
pub struct UnlockFromEvm<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        seeds = [b"bridge", config.token_mint.as_ref()],
        bump
    )]
    pub config: Account<'info, BridgeConfig>,

    #[account(
        init_if_needed,
        payer = relayer,
        space = 8 + ProcessedMessage::INIT_SPACE,
        seeds = [
            b"processed",
            &src_chain_id.to_le_bytes()[..],
            &nonce.to_le_bytes()
        ],
        bump
    )]
    pub processed_message: Account<'info, ProcessedMessage>,

    /// CHECK: PDA signer
    #[account(
        seeds = [b"vault", config.key().as_ref()],
        bump = config.vault_authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = token_vault.mint == config.token_mint,
        constraint = token_vault.owner == vault_authority.key()
    )]
    pub token_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = config.token_mint,
        token::authority = recipient
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    /// CHECK: used only for ATA constraint
    pub recipient: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
