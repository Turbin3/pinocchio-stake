use pinocchio::{
    account_info::AccountInfo, 
    program_error::ProgramError, 
    pubkey::Pubkey, 
    ProgramResult,
};
use pinocchio_log::log;
use crate::state::{
    get_stake_state, 
    relocate_lamports, 
    set_stake_state, 
    clock_from_account_info,
    MergeKind, 
    StakeAuthorize, 
    StakeHistorySysvar, 
    StakeStateV2
};

use crate::state::collect_signers;

// const MAX_SIGNERS: usize = 32;
use crate::consts::MAX_SIGNERS;


fn process_merge(
    accounts: &[AccountInfo],
) -> ProgramResult {
    let mut signers_arr = [Pubkey::default(); MAX_SIGNERS];
    let signers = collect_signers(accounts, &mut signers_arr)?;
    let account_info_iter = &mut accounts.iter();

    // native asserts: 4 accounts (2 sysvars)
    // let destination_stake_account_info = next_account_info(account_info_iter)?;
    // let source_stake_account_info = next_account_info(account_info_iter)?;
    // let clock_info = next_account_info(account_info_iter)?;
    // let _stake_history_info = next_account_info(account_info_iter)?;

    let [
        destination_stake_account_info,
        source_stake_account_info,
        clock_info,
        _stake_history_info,
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // other accounts
    // let _stake_authority_info = next_account_info(account_info_iter)?;

    let clock = clock_from_account_info(clock_info)?;
    let stake_history = &StakeHistorySysvar(clock.epoch);

    // check source stake account and destination stake account are not having same key
    if source_stake_account_info.key() == destination_stake_account_info.key() {
        return Err(ProgramError::InvalidArgument);
    }

    log!("Checking if destination stake is mergeable");
    let destination_merge_kind = MergeKind::get_if_mergeable( // MergeKind is a enum 
        &*get_stake_state(destination_stake_account_info)?,
        destination_stake_account_info.lamports(),
        &clock,
        stake_history,
    )?;

    // Authorized staker is allowed to split/merge accounts
    destination_merge_kind
        .meta() // implementation of state.rs
        .authorized
        .check(&signers_arr, StakeAuthorize::Staker) // implementation of state.rs
        .map_err(|_| ProgramError::MissingRequiredSignature)?;

    log!("Checking if source stake is mergeable");
    let source_merge_kind = MergeKind::get_if_mergeable(
        &*get_stake_state(source_stake_account_info)?,
        source_stake_account_info.lamports(),
        &clock,
        stake_history,
    )?;

    log!("Merging stake accounts");
    if let Some(merged_state) = destination_merge_kind.merge(source_merge_kind, &clock)? {
        set_stake_state(destination_stake_account_info, &merged_state)?;
    }

    // Source is about to be drained, deinitialize it's state
    set_stake_state(source_stake_account_info, &StakeStateV2::Uninitialized)?;

    // Drain the source stake account and transfer the lamports to the destination stake account
    relocate_lamports(
        source_stake_account_info, 
        destination_stake_account_info, 
        source_stake_account_info.lamports()
    )?;

    Ok(())
}