#![allow(unexpected_cfgs)]

use crate::instruction::{self, MyProgramInstruction};
use pinocchio::{
    account_info::AccountInfo, default_panic_handler, no_allocator, program_entrypoint,
    program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};
use pinocchio_log::log;

// This is the entrypoint for the program.
program_entrypoint!(process_instruction);
//Do not allocate memory.
no_allocator!();
// Use the no_std panic handler.
default_panic_handler!();

#[inline(always)]
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (ix_disc, instruction_data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match MyProgramInstruction::try_from(ix_disc)? {
        MyProgramInstruction::InitializeState => {
            log!("Ix:0");
            instruction::process_initilaize_state(accounts, instruction_data)
        }
        MyProgramInstruction::UpdateState => {
            log!("Ix:1");
            instruction::process_update_state(accounts, instruction_data)
        }
        MyProgramInstruction::Authorize => {
            log!("Ix:2");
            instruction::process_authorize(accounts, instruction_data)
        }
    }
}
