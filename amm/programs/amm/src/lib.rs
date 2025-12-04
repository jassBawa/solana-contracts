use anchor_lang::prelude::*;

declare_id!("5ozBoN2Ajg7YxTKgFgykmt23k2kHGKKQ9yievwpNRDKb");

mod constants;
mod errors;
mod instructions;
mod state;

#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
