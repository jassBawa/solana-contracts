use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorType {
    #[msg("Invalid fee value")]
    InvalidFee,

    #[msg("Invalid mint for the pool")]
    InvalidMint,

    #[msg("Depositing too little liquidituy")]
    DepositTooSmall,

    #[msg("Output is below the minimum expected")]
    OutputTooSmall,

    #[msg("Invariant does not hold")]
    InvariantViolated,

    #[msg("You don't have enough liqudity (LP tokens) ")]
    InsufficientLiquidity,

    #[msg("You dont have sufficient funds to withdraw")]
    ZeroWithdrawal,
}
