use anchor_lang::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("Nonce overflow")]
    NonceOverflow,

    #[msg("Amount must be greater than zero")]
    InvalidAmount,

    #[msg("You are not admin")]
    UnauthorizedAdmin,

    #[msg("Bridge is Paused")]
    BridgePaused,

    #[msg("Bridge locking is paused")]
    AlreadyPaused,

    #[msg("Bridge is not paused")]
    NotPaused,

    #[msg("Unauthorized caller")]
    Unauthorized,
    #[msg("Message already processed")]
    AlreadyProcessed,
}
