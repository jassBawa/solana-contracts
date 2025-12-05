use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, Mint, Token, TokenAccount, Transfer},
};
use fixed::types::I64F64;

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED, MINIMUM_LIQUIDITY},
    errors::ErrorType,
    state::{Amm, Pool},
};

pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
    let authority_bump = ctx.bumps.pool_authority;
    let seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    require!(
        ctx.accounts.depositer_account_token_lp.amount >= amount,
        ErrorType::InsufficientLiquidity
    );

    let supply = ctx.accounts.mint_liquidity.supply + MINIMUM_LIQUIDITY;

    // amount_a = (amount * pool_token_account_a.amount)/supply
    let amount_a = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(ctx.accounts.pool_token_account_a.amount))
        .unwrap()
        .checked_div(I64F64::from_num(supply))
        .unwrap()
        .floor()
        .to_num::<u64>();

    let amount_b = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(ctx.accounts.pool_token_account_b.amount))
        .unwrap()
        .checked_div(I64F64::from_num(supply))
        .unwrap()
        .floor()
        .to_num::<u64>();

    require!(amount_a > 0 && amount_b > 0, ErrorType::ZeroWithdrawal);

    // transfer from pool 1
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_token_account_a.to_account_info(),
                to: ctx.accounts.depositer_account_token_a.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount_a,
    )?;

    // transfer from pool 2
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_token_account_b.to_account_info(),
                to: ctx.accounts.depositer_account_token_b.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount_b,
    )?;

    // burn lp tokens
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint_liquidity.to_account_info(),
                from: ctx.accounts.depositer_account_token_lp.to_account_info(),
                authority: ctx.accounts.depositer.to_account_info(),
            },
        ),
        amount,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawLiquidity<'info> {
    #[account(
        seeds=[amm.id.as_ref()],
        bump
    )]
    pub amm: Box<Account<'info, Amm>>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        mut,
        seeds=[
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED
        ],
        bump
    )]
    pub mint_liquidity: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            pool.mint_a.as_ref(),
            pool.mint_b.as_ref()
        ],
        bump,
        has_one = amm,
        has_one = mint_a,
        has_one = mint_b
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: This is a PDA derived from known seeds; Anchor verifies the seeds and bump.
    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED
        ],
        bump
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,

    )]
    pub pool_token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,

    )]
    pub pool_token_account_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_liquidity,
        associated_token::authority = depositer
    )]
    pub depositer_account_token_lp: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = depositer
    )]
    pub depositer_account_token_a: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = depositer
    )]
    pub depositer_account_token_b: Account<'info, TokenAccount>,

    #[account(mut)]
    pub depositer: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
