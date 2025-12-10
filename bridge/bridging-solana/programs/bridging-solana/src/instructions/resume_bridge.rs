use anchor_lang::prelude::*;

use crate::{errors::ErrorCode, state::BridgeConfig};

pub fn resume_bridge(ctx: Context<ResumeBridge>) -> Result<()> {
    let config = &mut ctx.accounts.config;

    require!(config.paused, ErrorCode::NotPaused);

    config.paused = false;

    Ok(())
}

#[derive(Accounts)]
pub struct ResumeBridge<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bridge", config.token_mint.as_ref()],
        bump,
        constraint = admin.key() == config.admin.key() @ErrorCode::UnauthorizedAdmin
    )]
    pub config: Account<'info, BridgeConfig>,
}
