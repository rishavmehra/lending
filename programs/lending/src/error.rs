use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("insufficient funds")]
    InsufficientFunds,
    #[msg("Over borrowable Amount")]
    OverBorrowableAmount,
}
