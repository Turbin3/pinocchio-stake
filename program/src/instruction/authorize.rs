use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError, 
    pubkey::Pubkey, 
    ProgramResult,
};

use crate::state::{
    clock_from_account_info,
    collect_signers,
    do_authorize,
    StakeAuthorize,
};

pub fn process_authorize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let mut signers = [Pubkey::default(); 32];
    let _signers_len = collect_signers(accounts, &mut signers)?;

    // Expected accounts:
    // 0. [WRITE] Stake account to be updated
    // 1. [] Clock sysvar
    // 2. [SIGNER] The stake or withdraw authority
    // 3. Optional: [SIGNER] Lockup authority (if needed)
    let [stake_account_info, clock_info, _authority_info, rest @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Check for optional lockup authority
    let custodian = if !rest.is_empty() && rest[0].is_signer() {
        Some(rest[0].key())
    } else {
        None
    };

    // Extract new_authorized_pubkey and stake_authorize from instruction data
    if data.len() != 33 { // 32 bytes for Pubkey + 1 byte for StakeAuthorize
        return Err(ProgramError::InvalidInstructionData);
    }

    let new_authorized_pubkey = Pubkey::try_from(&data[0..32])
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    
    let stake_authorize = match data[32] {
        0 => StakeAuthorize::Staker,
        1 => StakeAuthorize::Withdrawer,
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    let clock = *clock_from_account_info(clock_info)?;
    
    // Perform authorization
    do_authorize(
        stake_account_info,
        &signers,
        &new_authorized_pubkey,
        stake_authorize,
        custodian,
        clock,
    )?;

    Ok(())
}
