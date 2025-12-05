use anchor_lang::prelude::*;

use crate::{errors::ErrorType, state::Amm};

pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
    let amm = &mut ctx.accounts.amm;

    amm.id = id;
    amm.admin = ctx.accounts.admin.key();
    amm.fee = fee;

    Ok(())
}

#[derive(Accounts)]
#[instruction(id: Pubkey, fee: u16)]
pub struct CreateAmm<'info> {
    #[account(
        init,
        payer = payer,
        space = Amm::LEN,
        seeds=[id.as_ref()],
        bump,
        constraint = fee < 10000 @ ErrorType::InvalidFee
    )]
    pub amm: Account<'info, Amm>,

    /// CHECK: Admin is provided by client; only its pubkey is used for authority.
    pub admin: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
