use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    errors::ErrorCode,
    state::{BridgeConfig, LockRecord},
};

pub fn lock_tokens(
    ctx: Context<LockTokens>,
    amount: u64,
    destination_address: [u8; 20],
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let user = &ctx.accounts.user;
    let user_token_ata = &ctx.accounts.user_token_account;
    let token_vault = &ctx.accounts.token_vault;

    require!(config.paused, ErrorCode::BridgePaused);
    require!(amount > 0, ErrorCode::InvalidAmount);

    // transfer from user to vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: user_token_ata.to_account_info(),
            to: token_vault.to_account_info(),
            authority: user.to_account_info(),
        },
    );

    token::transfer(transfer_ctx, amount)?;

    // update lock record
    let current_nonce = config.nonce;
    let lock_record = &mut ctx.accounts.lock_record;
    lock_record.config = config.key();
    lock_record.nonce = current_nonce;
    lock_record.user = user.key();
    lock_record.amount = amount;
    lock_record.destination_address = destination_address;
    lock_record.created_at_slot = Clock::get()?.slot;

    // increment global nonce
    config.nonce = config
        .nonce
        .checked_add(1)
        .ok_or_else(|| error!(ErrorCode::NonceOverflow))?;

    emit!(BridgeLockEvent {
        config: config.key(),
        nonce: current_nonce,
        user: user.key(),
        amount,
        destination_address,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct LockTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds=[b"bridge", config.token_mint.as_ref()],
        bump
    )]
    pub config: Account<'info, BridgeConfig>,

    /// CHECK: PDA derived authority; only used as token authority
    #[account(
        seeds=[b"vault", config.key().as_ref()],
        bump = config.vault_authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = user_token_account.mint == config.token_mint,
        constraint = user_token_account.owner == user.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = token_vault.mint == config.token_mint,
        constraint = token_vault.owner == vault_authority.key()
    )]
    pub token_vault: Account<'info, TokenAccount>,

    // everytime new record pda is created
    #[account(
        init,
        payer = user,
        space = 8 + LockRecord::INIT_SPACE,
        seeds = [b"lock", config.key().as_ref(), &config.nonce.to_le_bytes()],
        bump
    )]
    pub lock_record: Account<'info, LockRecord>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[event]
pub struct BridgeLockEvent {
    config: Pubkey,
    nonce: u64,
    user: Pubkey,
    amount: u64,
    destination_address: [u8; 20],
}
