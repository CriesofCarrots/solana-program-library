//! Instruction types

use crate::error::CpiError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};
use std::convert::TryInto;
use std::mem::size_of;

/// Instructions supported by the program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum CpiInstruction {
    /// Invokes system transfer via cpi
    InvokedTransfer {
        /// Amount to transfer, in lamports
        amount: u64,
    },
}

impl CpiInstruction {
    /// Unpacks a byte buffer into a [CpiInstruction](enum.CpiInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use CpiError::InvalidInstruction;

        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                Self::InvokedTransfer { amount }
            }

            _ => return Err(CpiError::InvalidInstruction.into()),
        })
    }

    /// Packs a [CpiInstruction](enum.CpiInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            &Self::InvokedTransfer { amount } => {
                buf.push(0);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
        };
        buf
    }
}

/// Creates a `InvokedTransfer` instruction.
pub fn invoked_transfer(
    program_id: &Pubkey,
    source_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = CpiInstruction::InvokedTransfer { amount }.pack();

    let mut accounts = Vec::with_capacity(2);
    accounts.push(AccountMeta::new(*source_pubkey, true));
    accounts.push(AccountMeta::new(*destination_pubkey, false));
    accounts.push(AccountMeta::new_readonly(system_program::id(), false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
