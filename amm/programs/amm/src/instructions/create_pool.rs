use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED},
    state::{Amm, Pool},
};

pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    pool.amm = ctx.accounts.amm.key();
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();
    Ok(())
}

#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(
        seeds=[amm.id.as_ref()],
        bump
    )]
    pub amm: Account<'info, Amm>,

    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
        ],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: PDA, safe because seeds are verified
    #[account(
        seeds=[
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump
    )]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority
    )]
    pub mint_liqudity: Account<'info, Mint>, // for minting lptokens

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority
    )]
    pub pool_account_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority
    )]
    pub pool_account_b: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
