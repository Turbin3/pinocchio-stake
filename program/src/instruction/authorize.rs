use pinocchio::{
    account_info:: AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};
use pinocchio_log::log;

use crate::{
    error::MyProgramError,
    state::{
        utils::{load_acc_mut, load_ix_data, DataLen},
        StakeAuthorize,
        MyState,
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuthorizeIxData {
    pub new_authorized_pubkey: Pubkey,
    pub stake_authorize: StakeAuthorize,
}

impl DataLen for AuthorizeIxData {
    const LEN: usize = core::mem::size_of::<AuthorizeIxData>();
}

pub fn process_authorize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let mut account_info_iter = accounts.iter();
    
    let stake_account = account_info_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
        
    let clock_sysvar = account_info_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let authority = account_info_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    
    // Optional custodian (lockup authority)
    let custodian = match account_info_iter.next() {
        Some(acc) => Some(acc),
        None => None
    };
    
    if !stake_account.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }
    
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    let ix_data = unsafe { load_ix_data::<AuthorizeIxData>(data)? };
    
    // This would need to be replaced with the actual stake state
    let mut stake_state = unsafe { load_acc_mut::<MyState>(stake_account.borrow_mut_data_unchecked())? };
    
    // Check if we're updating withdrawer and need custodian
    if ix_data.stake_authorize == StakeAuthorize::Withdrawer {
        // Here you would check if lockup is active and require custodian signature
        if let Some(custodian) = custodian {
            if !custodian.is_signer() {
                log!("Error: custodian must sign when changing withdrawer before lockup expiration");
                return Err(ProgramError::MissingRequiredSignature);
            }
        }
    }
    
    // Update the authority based on the stake_authorize type
    match ix_data.stake_authorize {
        StakeAuthorize::Staker => {
            log!("Updating stake authority to {}", &ix_data.new_authorized_pubkey);
            // // For example, if you added a staker field to MyState:
            // stake_state.staker = ix_data.new_authorized_pubkey;
        }
        StakeAuthorize::Withdrawer => {
            log!("Updating withdraw authority to {}", &ix_data.new_authorized_pubkey);
            // For example, if you added a withdrawer field to MyState:
            // stake_state.withdrawer = ix_data.new_authorized_pubkey;
        }
    }
    
    Ok(())
}
