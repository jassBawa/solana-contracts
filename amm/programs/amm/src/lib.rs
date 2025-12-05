use anchor_lang::prelude::*;

declare_id!("5ozBoN2Ajg7YxTKgFgykmt23k2kHGKKQ9yievwpNRDKb");

mod constants;
mod errors;
mod instructions;
mod state;

#[program]
pub mod swap_example {
    // needed pub to export this instructions in idl file
    pub use super::instructions::*;
    pub use super::*;

    pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
        instructions::create_amm(ctx, id, fee)
    }

    pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
        instructions::create_pool(ctx)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        instructions::deposit_liquidity(ctx, amount_a, amount_b)
    }

    pub fn swap_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        instructions::swap_tokens_for_tokens(ctx, swap_a, input_amount, min_output_amount)
    }

    pub fn withdraw_liqudity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
        instructions::withdraw_liquidity(ctx, amount)
    }
}
