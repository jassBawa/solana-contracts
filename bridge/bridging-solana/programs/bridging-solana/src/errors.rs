use anchor_lang::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("Nonce overflow")]
    NonceOverflow,

    #[msg("Amount must be greater than zero")]
    InvalidAmount,
}
