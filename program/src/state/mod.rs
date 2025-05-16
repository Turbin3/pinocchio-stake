
pub mod authorized;
pub mod authorized_checked_with_seed;
pub mod delegation;
pub mod lockup;
pub mod merge;
pub mod meta;
pub mod redelegate_state;
pub mod stake;
pub mod stake_authorize;
pub mod stake_flags;
pub mod stake_history;
pub mod stake_history_sysvar;
pub mod stake_state_v2;
pub mod vote_state_v3;
pub mod authorized_voters;
pub mod utils;

pub use authorized::*;
pub use delegation::*;
pub use vote_state_v3::*;
pub use authorized_voters::*;
pub use lockup::*;
pub use merge::*;
pub use meta::*;
pub use authorized_checked_with_seed::*;
use pinocchio::{
    account_info::{ AccountInfo, Ref, RefMut },
    program_error::ProgramError,
    ProgramResult,
};
pub use stake::*;
pub use stake_authorize::*;
pub use stake_flags::*;
pub use stake_history::*;
pub use stake_history_sysvar::*;
pub use stake_state_v2::*;
pub use utils::*;

use crate::consts::VOTE_PROGRAM_ID;
pub use redelegate_state::*;

pub type Epoch = [u8; 8]; //u64
pub type UnixTimestamp = [u8; 8]; //i64;

pub fn get_stake_state(
    stake_account_info: &AccountInfo
) -> Result<Ref<StakeStateV2>, ProgramError> {
    if stake_account_info.is_owned_by(&crate::ID) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    StakeStateV2::from_account_info(stake_account_info)
}

pub fn set_stake_state(
    stake_account_info: &AccountInfo,
    new_state: &StakeStateV2
) -> Result<(), ProgramError> {
    let new_state_size = core::mem::size_of::<StakeStateV2>();
    let data = stake_account_info.try_borrow_mut_data()?;
    if data.len() < new_state_size {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let mut new_state_bytes = [0u8; core::mem::size_of::<StakeStateV2>()];
    new_state_bytes.copy_from_slice(unsafe {
        core::slice::from_raw_parts(new_state as *const StakeStateV2 as *const u8, new_state_size)
    });
    stake_account_info.try_borrow_mut_data()?.copy_from_slice(&new_state_bytes);
    Ok(())
}

/// # Safety
///
/// The caller must ensure that it is safe to borrow the account data – e.g., there are
/// no mutable borrows of the account data.
pub unsafe fn get_stake_state_unchecked(
    stake_account_info: &AccountInfo
) -> Result<&StakeStateV2, ProgramError> {
    if stake_account_info.owner() != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    StakeStateV2::from_account_info_unchecked(stake_account_info)
}

pub fn try_get_stake_state_mut(
    stake_account_info: &AccountInfo
) -> Result<RefMut<StakeStateV2>, ProgramError> {
    if stake_account_info.is_owned_by(&crate::ID) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    StakeStateV2::try_from_account_info_mut(stake_account_info)
}

// dont call this "move" because we have an instruction MoveLamports
pub fn relocate_lamports(
    source_account_info: &AccountInfo,
    destination_account_info: &AccountInfo,
    lamports: u64
) -> ProgramResult {
    {
        let mut source_lamports = source_account_info.try_borrow_mut_lamports()?;
        *source_lamports = source_lamports
            .checked_sub(lamports)
            .ok_or(ProgramError::InsufficientFunds)?;
    }

    {
        let mut destination_lamports = destination_account_info.try_borrow_mut_lamports()?;
        *destination_lamports = destination_lamports
            .checked_add(lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    Ok(())
}

pub fn get_vote_state(vote_account_info: &AccountInfo) -> Result<Ref<VoteState>, ProgramError> {
    if vote_account_info.is_owned_by(&VOTE_PROGRAM_ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let vote_state = VoteState::from_account_info(vote_account_info)?;
    return Ok(vote_state);
}

pub fn checked_add(a: [u8; 8], b: [u8; 8]) -> Result<[u8; 8], ProgramError> {
    let a_u64 = u64::from_le_bytes(a);
    let b_u64 = u64::from_le_bytes(b);
    a_u64.checked_add(b_u64)
        .map(|result| result.to_le_bytes())
        .ok_or(ProgramError::InsufficientFunds)
}
