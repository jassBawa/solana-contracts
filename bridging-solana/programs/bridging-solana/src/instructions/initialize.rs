use crate::state::BridgeConfig;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

pub fn initialize_bridge(
    ctx: Context<InitializeBridge>,
    destination_chain_id: u64,
    destination_bridge: [u8; 20],
    relayer: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    config.admin = ctx.accounts.admin.key();
    config.token_mint = ctx.accounts.token_mint.key();
    config.vault_authority_bump = ctx.bumps.vault_authority;
    config.nonce = 0;
    config.destination_chain_id = destination_chain_id;
    config.destination_bridge = destination_bridge;
    config.relayer_pubkey = relayer;

    Ok(())
}

#[derive(Accounts)]
#[instruction(destination_chain_id: u64, destination_bridge: [u8; 20])]
pub struct InitializeBridge<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + BridgeConfig::INIT_SPACE,
        seeds=[b"bridge", token_mint.key().as_ref()],
        bump
    )]
    pub config: Account<'info, BridgeConfig>,

    /// CHECK: Vault authority doesn't need to be checked
    #[account(
        seeds=[b"vault", config.key().as_ref()],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority
    )]
    pub token_vault: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
