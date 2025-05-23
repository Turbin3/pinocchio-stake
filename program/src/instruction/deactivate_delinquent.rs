use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{ clock::Clock, Sysvar },
    ProgramResult,
};
use crate::{
    error::StakeError,
    state::{
        acceptable_reference_epoch_credits,
        eligible_for_deactivate_delinquent,
        get_stake_state,
        get_vote_state,
        next_account_info,
        set_stake_state,
        StakeStateV2,
    },
};

pub fn process_deactivate_delinquent(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // native asserts: 3 accounts
    let stake_account_info = next_account_info(account_info_iter)?;
    let delinquent_vote_account_info = next_account_info(account_info_iter)?;
    let reference_vote_account_info = next_account_info(account_info_iter)?;

    let clock = Clock::get()?;

    let delinquent_vote_state = get_vote_state(delinquent_vote_account_info)?;
    let reference_vote_state = get_vote_state(reference_vote_account_info)?;

    if
        !acceptable_reference_epoch_credits(
            &reference_vote_state.epoch_credits,
            clock.epoch.to_le_bytes()
        )
    {
        return Err(StakeError::InsufficientReferenceVotes.into());
    }
    match *get_stake_state(stake_account_info)? {
        StakeStateV2::Stake(meta, mut stake, stake_flags) => {
            if stake.delegation.voter_pubkey != *delinquent_vote_account_info.key() {
                return Err(StakeError::VoteAddressMismatch.into());
            }

            // Deactivate the stake account if its delegated vote account has never voted or
            // has not voted in the last
            // `MINIMUM_DELINQUENT_EPOCHS_FOR_DEACTIVATION`
            if
                eligible_for_deactivate_delinquent(
                    &delinquent_vote_state.epoch_credits,
                    clock.epoch.to_le_bytes()
                )
            {
                stake.deactivate(clock.epoch.to_le_bytes())?;
                set_stake_state(stake_account_info, &StakeStateV2::Stake(meta, stake, stake_flags));
            } else {
                return Err(StakeError::MinimumDelinquentEpochsForDeactivationNotMet.into());
            }
        }
        _ => {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    Ok(())
}
