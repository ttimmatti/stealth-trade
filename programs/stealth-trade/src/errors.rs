use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Not Admin")]
    NotAdmin,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Paused")]
    Paused,
    #[msg("Max Positions Reached")]
    MaxPositionsReached,
    #[msg("Slippage Exceeded")]
    SlippageExceeded,
    #[msg("Insufficient Balance")]
    InsufficientBalance,
    #[msg("Position Not Found")]
    PositionNotFound,
    #[msg("No Liquidity In Pool")]
    NoLiquidityInPool,
    #[msg("Insufficient Pool Balance")]
    InsufficientPoolBalance,
    #[msg("Pool Not Active")]
    PoolNotActive,
    #[msg("Liquidity Pool Overflow")]
    LiquidityPoolOverflow,
    #[msg("Invalid ER Validator")]
    InvalidERValidator,
    #[msg("Session Not Authenticated")]
    SessionNotAuthenticated,
    #[msg("Session Not Authorized")]
    SessionNotAuthorized,
    #[msg("User Not Authorized")]
    UserNotAuthorized,
}
