//! Error types

use num_derive::FromPrimitive;
use solana_sdk::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum CpiError {
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,
}
impl From<CpiError> for ProgramError {
    fn from(e: CpiError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for CpiError {
    fn type_of() -> &'static str {
        "CpiError"
    }
}
