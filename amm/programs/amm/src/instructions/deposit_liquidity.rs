use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};
use fixed::types::I64F64;

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED, MINIMUM_LIQUIDITY},
    errors::ErrorType,
    state::Pool,
};

pub fn deposit_liquidity(
    ctx: Context<DepositLiquidity>,
    amount_a: u64,
    amount_b: u64,
) -> Result<()> {
    if amount_a == 0 || amount_b == 0 {
        return err!(ErrorType::DepositTooSmall);
    }

    let mut amount_a = if amount_a > ctx.accounts.depositer_mint_a_ata.amount {
        ctx.accounts.depositer_mint_a_ata.amount
    } else {
        amount_a
    };
    let mut amount_b = if amount_b > ctx.accounts.depositer_mint_b_ata.amount {
        ctx.accounts.depositer_mint_b_ata.amount
    } else {
        amount_b
    };

    let pool_a = &ctx.accounts.pool_account_a;
    let pool_b = &ctx.accounts.pool_account_b;

    let reserve_a = pool_a.amount;
    let reserve_b = pool_b.amount;

    // lp total supply
    let lp_supply = ctx.accounts.mint_liquidity.supply;

    let pool_creation = reserve_a == 0 && reserve_b == 0 && lp_supply == 0;

    if !pool_creation {
        // ideal_b = (amount_a * reserve_b)/reserve_a
        let ideal_b = I64F64::from_num(amount_a)
            .checked_mul(I64F64::from_num(reserve_b))
            .unwrap()
            .checked_div(I64F64::from_num(reserve_a))
            .unwrap()
            .to_num::<u64>();

        if ideal_b <= amount_b {
            // user is trying to give too much b than needed
            amount_b = ideal_b;
        } else {
            // ideal_a = (amount_b * reserve_a)/ reserve_b
            let ideal_a = I64F64::from_num(amount_b)
                .checked_mul(I64F64::from_num(reserve_a))
                .unwrap()
                .checked_div(I64F64::from_num(reserve_b))
                .unwrap()
                .to_num::<u64>();
            amount_a = ideal_a
        }
    }

    let mut liquidity: u64;

    if pool_creation {
        liquidity = I64F64::from_num(amount_a)
            .checked_mul(I64F64::from_num(amount_b))
            .unwrap()
            .sqrt()
            .to_num::<u64>();

        if liquidity < MINIMUM_LIQUIDITY {
            return err!(ErrorType::DepositTooSmall);
        }

        liquidity -= MINIMUM_LIQUIDITY;
    } else {
        let liquidity_from_a = I64F64::from_num(amount_a)
            .checked_mul(I64F64::from_num(lp_supply))
            .unwrap()
            .checked_div(I64F64::from_num(reserve_a))
            .unwrap()
            .to_num::<u64>();

        let liquidity_from_b = I64F64::from_num(amount_b)
            .checked_mul(I64F64::from_num(lp_supply))
            .unwrap()
            .checked_div(I64F64::from_num(reserve_b))
            .unwrap()
            .to_num::<u64>();

        liquidity = liquidity_from_a.min(liquidity_from_b);

        if liquidity == 0 {
            return err!(ErrorType::DepositTooSmall);
        }
    }

    // adding tokens to there respective vaults
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositer_mint_a_ata.to_account_info(),
                to: ctx.accounts.pool_account_a.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount_a,
    )?;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositer_mint_b_ata.to_account_info(),
                to: ctx.accounts.pool_account_b.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount_b,
    )?;

    // minting the lp tokens
    let authority_bump = ctx.bumps.pool_authority;
    let seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_liquidity.to_account_info(),
                to: ctx.accounts.depositer_mint_lp_ata.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        liquidity,
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct DepositLiquidity<'info> {
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        mut,
        seeds=[
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref()
        ],
        bump,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: Read only authority
    #[account(
        mut,
        seeds=[
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED
        ],
        bump
    )]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds=[
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED
        ],
        bump,
        mint::authority = pool_authority
    )]
    pub mint_liquidity: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority
    )]
    pub pool_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority
    )]
    pub pool_account_b: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_liquidity,
        associated_token::authority = depositor,
    )]
    pub depositer_mint_lp_ata: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_a, associated_token::authority = depositor)]
    pub depositer_mint_a_ata: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_b, associated_token::authority = depositor)]
    pub depositer_mint_b_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
