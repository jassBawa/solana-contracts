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

    pub fn unlock_from_evm(
        ctx: Context<UnlockFromEvm>,
        src_chain_id: u64,
        nonce: u64,
        amount: u64,
    ) -> Result<()> {
        instructions::unlock_from_evm(ctx, src_chain_id, nonce, amount)
    }

    pub fn pause_bridge(
        ctx: Context<PauseBridge>
    ) -> Result<()>{
        instructions::pause_bridge(ctx)
    }

    pub fn resume_bridge(
        ctx: Context<ResumeBridge>
    ) -> Result<()>{
        instructions::resume_bridge(ctx)
    }
}
