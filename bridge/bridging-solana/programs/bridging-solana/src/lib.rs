use anchor_lang::prelude::*;

declare_id!("F5qk3bMoRNyZao5RciKt7X5BN44wg93p6ExE5qwSi4Ww");

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

#[program]
pub mod bridging_solana {
    use super::*;

    pub fn initialize(
        ctx: Context<InitializeBridge>,
        destination_chain_id: u64,
        destination_bridge: [u8; 20],
        relayer: Pubkey,
    ) -> Result<()> {
        instructions::initialize_bridge(ctx, destination_chain_id, destination_bridge, relayer)
    }

    pub fn lock_tokens(
        ctx: Context<LockTokens>,
        amount: u64,
        destination_address: [u8; 20],
    ) -> Result<()> {
        instructions::lock_tokens(ctx, amount, destination_address)
    }
}
