use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Amm {
    pub id: Pubkey,
    pub admin: Pubkey,
    pub fee: u16,
}

impl Amm {
    pub const LEN: usize = 8 + 32 + 32 + 2;
}

#[account]
#[derive(Default)]
pub struct Pool {
    pub amm: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
}

impl Pool {
    pub const LEN: usize = 8 + 32 + 32 + 32;
}
