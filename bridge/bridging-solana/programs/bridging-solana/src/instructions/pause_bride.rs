use crate::{errors::ErrorCode, state::BridgeConfig};
use anchor_lang::prelude::*;

pub fn pause_bridge(ctx: Context<PauseBridge>) -> Result<()> {
    let config = &mut ctx.accounts.config;

    require!(!config.paused, ErrorCode::AlreadyPaused);

    config.paused = true;

    Ok(())
}

#[derive(Accounts)]
pub struct PauseBridge<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds=[b"bridge", config.token_mint.as_ref()],
        bump,
        constraint = config.admin == admin.key() @ErrorCode::UnauthorizedAdmin
    )]
    pub config: Account<'info, BridgeConfig>,
}
