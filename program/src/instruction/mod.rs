use pinocchio::program_error::ProgramError;

// pub mod authorize_with_seed
pub mod authorized_checked;
pub mod move_lamports;
pub mod redelegate;
pub mod set_lockup;
pub mod split;
pub mod delegate_stake;
pub mod merge;

// pub use authorize_with_seed::*;
pub use authorized_checked::*;
pub use move_lamports::*;
pub use redelegate::*;
pub use set_lockup::*;
pub use split::*;
pub use delegate_stake::*;
pub use merge::*;

#[repr(u8)]
pub enum StakeInstruction {
    Initialize,
    Authorize,
    DelegateStake,
    Split,
    Withdraw,
    Deactivate,
    SetLockup,
    Merge,
    AuthorizeWithSeed,
    InitializeChecked,
    AuthorizeChecked,
    AuthorizeCheckedWithSeed,
    SetLockupChecked,
    GetMinimumDelegation,
    DeactivateDelinquent,
    #[deprecated(since = "2.1.0", note = "Redelegate will not be enabled")]
    Redelegate,
    MoveStake,
    MoveLamports,
}

impl TryFrom<&u8> for StakeInstruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(StakeInstruction::Initialize),
            1 => Ok(StakeInstruction::Authorize),
            2 => Ok(StakeInstruction::DelegateStake),
            3 => Ok(StakeInstruction::Split),
            4 => Ok(StakeInstruction::Withdraw),
            5 => Ok(StakeInstruction::Deactivate),
            6 => Ok(StakeInstruction::SetLockup),
            7 => Ok(StakeInstruction::Merge),
            8 => Ok(StakeInstruction::AuthorizeWithSeed),
            9 => Ok(StakeInstruction::InitializeChecked),
            10 => Ok(StakeInstruction::AuthorizeChecked),
            11 => Ok(StakeInstruction::AuthorizeCheckedWithSeed),
            12 => Ok(StakeInstruction::SetLockupChecked),
            13 => Ok(StakeInstruction::GetMinimumDelegation),
            14 => Ok(StakeInstruction::DeactivateDelinquent),
            #[allow(deprecated)]
            15 => Ok(StakeInstruction::Redelegate),
            16 => Ok(StakeInstruction::MoveStake),
            17 => Ok(StakeInstruction::MoveLamports),
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
    }
}
