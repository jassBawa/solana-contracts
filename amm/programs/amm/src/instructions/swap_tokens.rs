use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};
use fixed::types::I64F64;

use crate::{
    constants::AUTHORITY_SEED,
    errors::ErrorType,
    state::{Amm, Pool},
};

pub fn swap_tokens_for_tokens(
    ctx: Context<SwapExactTokensForTokens>,
    swap_a: bool,
    input_amount: u64,
    min_output_amount: u64,
) -> Result<()> {
    let input = if swap_a && input_amount > ctx.accounts.trader_account_a.amount {
        ctx.accounts.trader_account_a.amount
    } else if !swap_a && input_amount > ctx.accounts.trader_account_b.amount {
        ctx.accounts.trader_account_b.amount
    } else {
        input_amount
    };

    let amm = &ctx.accounts.amm;
    let taxed_input = input - input * amm.fee as u64 / 10000;

    let pool_a = &ctx.accounts.pool_account_a;
    let pool_b = &ctx.accounts.pool_account_b;

    let output = if swap_a {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_b.amount))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_a.amount)
                    .checked_add(I64F64::from_num(taxed_input))
                    .unwrap(),
            )
            .unwrap()
    } else {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_a.amount))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_b.amount)
                    .checked_add(I64F64::from_num(taxed_input))
                    .unwrap(),
            )
            .unwrap()
    }
    .to_num::<u64>();

    if output < min_output_amount {
        return err!(ErrorType::OutputTooSmall);
    }

    let invariant = pool_a.amount * pool_b.amount;

    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];

    // swapping with a
    if swap_a {
        // transfer from user to pool
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trader_account_a.to_account_info(),
                    to: ctx.accounts.pool_account_a.to_account_info(),
                    authority: ctx.accounts.trader.to_account_info(),
                },
            ),
            input,
        )?;

        // transfer from pool to user
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_account_b.to_account_info(),
                    to: ctx.accounts.trader_account_b.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            output,
        )?;
    }
    // swapping with b
    else {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trader_account_b.to_account_info(),
                    to: ctx.accounts.pool_account_b.to_account_info(),
                    authority: ctx.accounts.trader.to_account_info(),
                },
            ),
            input,
        )?;

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_account_a.to_account_info(),
                    to: ctx.accounts.trader_account_a.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            output,
        )?;
    }

    msg!(
        "Traded {} tokens ({} after fees) for {}",
        input,
        taxed_input,
        output
    );

    ctx.accounts.pool_account_a.reload()?;
    ctx.accounts.pool_account_b.reload()?;
    if invariant > ctx.accounts.pool_account_a.amount * ctx.accounts.pool_account_a.amount {
        return err!(ErrorType::InvariantViolated);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct SwapExactTokensForTokens<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump
    )]
    pub amm: Account<'info, Amm>,

    #[account(
          seeds = [
              pool.amm.as_ref(),
              pool.mint_a.key().as_ref(),
              pool.mint_b.key().as_ref(),
          ],
          bump,
          has_one = amm,
          has_one = mint_a,
          has_one = mint_b,
      )]
    pub pool: Account<'info, Pool>,

    /// CHECK: PDA used only as authority
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
    pub pool_authority: AccountInfo<'info>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_b: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = trader
    )]
    pub trader_account_a: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = trader
    )]
    pub trader_account_b: Account<'info, TokenAccount>,

    #[account(mut)]
    pub trader: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
