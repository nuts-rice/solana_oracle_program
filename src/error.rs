// inside error.rs
use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum OracleError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,

    /// Not Rent Exempt
    #[error("Not Rent Exempt")]
    NotRentExempt,

    /// InvalidMintData
    #[error("Invalid Mint Data")]
    InvalidMintData,

    /// Mismatched PDA Accounts For AMMs
    #[error("MismatchedPDAAccountsForAMMs")]
    MismatchedPDAAccountsForAMMs,
}

impl From<OracleError> for ProgramError {
    fn from(e: OracleError) -> Self {
        ProgramError::Custom(e as u32)
    }
}