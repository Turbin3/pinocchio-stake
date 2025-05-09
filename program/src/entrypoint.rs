#![allow(unexpected_cfgs)]

use crate::instruction::{self, StakeInstruction};
use pinocchio::{
    account_info::AccountInfo, default_panic_handler, no_allocator, program_entrypoint, program_error::ProgramError, pubkey::Pubkey, ProgramResult
};

// This is the entrypoint for the program.
program_entrypoint!(process_instruction);
//Do not allocate memory.
no_allocator!();
// Use the no_std panic handler.
default_panic_handler!();

#[inline(always)]
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // convenience so we can safely use id() everywhere
    if *program_id != crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (ix_disc, instruction_data) = instruction_data
        .split_first_chunk::<4>()
        .ok_or(ProgramError::InvalidInstructionData)?;

    // Second variant, test CUs usage
    // let (ix_disc, instruction_data) = instruction_data
    //     .split_at_checked(4)
    //     .ok_or(ProgramError::InvalidInstructionData)?;

    let instruction = StakeInstruction::try_from(&ix_disc[0])?;

    // TODO: add check for epoch_rewards_active
    // let epoch_rewards_active = EpochRewards::get()
    //         .map(|epoch_rewards| epoch_rewards.active)
    //         .unwrap_or(false);
    // if epoch_rewards_active && !matches!(instruction, StakeInstruction::GetMinimumDelegation) {
    //     return Err(StakeError::EpochRewardsActive.into());
    // }

    match instruction {
        StakeInstruction::Initialize => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: Initialize");

            todo!()
        }
        StakeInstruction::Authorize => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: Authorize");

            todo!()
        }
        StakeInstruction::DelegateStake => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: DelegateStake");

            todo!()
        }
        StakeInstruction::Split => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: Split");

            todo!()
        }
        StakeInstruction::Withdraw => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: Withdraw");

            todo!()
        }
        StakeInstruction::Deactivate => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: Deactivate");

            todo!()
        }
        StakeInstruction::SetLockup => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: SetLockup");

            instruction::process_set_lockup(accounts, instruction_data)
        }
        StakeInstruction::Merge => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: Merge");
            
            todo!()
        }
        StakeInstruction::AuthorizeWithSeed => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: AuthorizeWithSeed");

            todo!()
        }
        StakeInstruction::InitializeChecked => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: InitializeChecked");

            todo!()
        }
        StakeInstruction::AuthorizeChecked => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: AuthorizeChecked");

            todo!()
        }
        StakeInstruction::AuthorizeCheckedWithSeed => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: AuthorizeCheckedWithSeed");

            todo!()
        }
        StakeInstruction::SetLockupChecked => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: SetLockupChecked");

            todo!()
        }
        StakeInstruction::GetMinimumDelegation => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: GetMinimumDelegation");

            todo!()
        }
        StakeInstruction::DeactivateDelinquent => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: DeactivateDelinquent");

            todo!()
        }
        #[allow(deprecated)]
        StakeInstruction::Redelegate => Err(ProgramError::InvalidInstructionData),
        // NOTE we assume the program is going live after `move_stake_and_move_lamports_ixs` is
        // activated
        StakeInstruction::MoveStake => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: MoveStake");

            todo!()
        }
        StakeInstruction::MoveLamports => {
            #[cfg(feature = "logging")]
            pinocchio::msg!("Instruction: MoveLamports");

            instruction::process_move_lamports(accounts)
        }
    }
}
