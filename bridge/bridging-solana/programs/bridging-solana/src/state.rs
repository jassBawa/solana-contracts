use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct BridgeConfig {
    pub admin: Pubkey,
    pub token_mint: Pubkey,
    pub vault_authority_bump: u8,
    pub nonce: u64,
    pub destination_chain_id: u64,
    pub destination_bridge: [u8; 20],
    pub relayer_pubkey: Pubkey,
    pub paused: bool,
}

#[account]
#[derive(InitSpace)]
pub struct LockRecord {
    pub config: Pubkey,
    pub nonce: u64,
    pub user: Pubkey,
    pub amount: u64,
    pub destination_address: [u8; 20],
    pub created_at_slot: u64,
}

#[account]
#[derive(InitSpace)]
pub struct ProcessedMessage {
    pub executed: bool,
}
