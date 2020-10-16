//! Program state processor

#![cfg(feature = "program")]

use crate::{error::CpiError, instruction::CpiInstruction};
use num_traits::FromPrimitive;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    info,
    program::invoke,
    program_error::PrintProgramError,
    pubkey::Pubkey,
    system_instruction,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Processes an [InitializeMint](enum.CpiInstruction.html) instruction.
    pub fn process_invoked_transfer(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let system_program_account_info = next_account_info(account_info_iter)?;

        invoke(
            &system_instruction::transfer(source_account_info.key, dest_account_info.key, amount),
            &[
                source_account_info.clone(),
                dest_account_info.clone(),
                system_program_account_info.clone(),
            ],
        )?;
        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = CpiInstruction::unpack(input)?;

        match instruction {
            CpiInstruction::InvokedTransfer { amount } => {
                info!("Instruction: InvokedTransfer");
                Self::process_invoked_transfer(accounts, amount)
            }
        }
    }
}

impl PrintProgramError for CpiError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            CpiError::InvalidInstruction => info!("Error: Invalid instruction"),
        }
    }
}
