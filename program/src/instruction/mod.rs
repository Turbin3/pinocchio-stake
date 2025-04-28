use pinocchio::program_error::ProgramError;

pub mod initialize_mystate;
pub mod update_mystate;
pub mod authorize; // Add this line

pub use initialize_mystate::*;
pub use update_mystate::*;
pub use authorize::*; // Add this line

#[repr(u8)]
pub enum MyProgramInstruction {
    InitializeState,
    UpdateState,
    Authorize, // Add this variant
}

impl TryFrom<&u8> for MyProgramInstruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(MyProgramInstruction::InitializeState),
            1 => Ok(MyProgramInstruction::UpdateState),
            2 => Ok(MyProgramInstruction::Authorize), // Add this match arm
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

mod idl_gen {
    #[derive(shank::ShankInstruction)]
    enum _MyProgramInstruction {
        #[account(0, writable, signer, name = "payer_acc", desc = "Fee payer account")]
        #[account(1, writable, name = "state_acc", desc = "New State account")]
        #[account(2, name = "sysvar_rent_acc", desc = "Sysvar rent account")]
        #[account(3, name = "system_program_acc", desc = "System program account")]
        InitializeState,
        #[account(0, writable, signer, name = "payer_acc", desc = "Fee payer account")]
        #[account(1, writable, name = "state_acc", desc = "State account")]
        UpdateState,
        #[account(0, writable, name = "stake_account", desc = "Stake account to be updated")]
        #[account(1, name = "clock_sysvar", desc = "Clock sysvar")]
        #[account(2, signer, name = "authority", desc = "The stake or withdraw authority")]
        #[account(3, signer, optional, name = "custodian", desc = "Optional lockup authority")]
        Authorize,
    }
}
