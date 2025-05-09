use pinocchio::{
    account_info::{ AccountInfo, Ref },
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult, SUCCESS,
};

extern crate alloc;
use super::{
    get_stake_state, try_get_stake_state_mut, Delegation, Meta, Stake, StakeAuthorize, StakeHistorySysvar, StakeStateV2, VoteState, DEFAULT_WARMUP_COOLDOWN_RATE
};
use crate::{
    consts::{
        FEATURE_STAKE_RAISE_MINIMUM_DELEGATION_TO_1_SOL, LAMPORTS_PER_SOL, MAX_SIGNERS,
        NEW_WARMUP_COOLDOWN_RATE,
    },
    helpers::MergeKind,
};
use crate::{consts::{
    CLOCK_ID, HASH_BYTES, MAX_BASE58_LEN, PERPETUAL_NEW_WARMUP_COOLDOWN_RATE_EPOCH
}, error::StakeError};
use alloc::boxed::Box;
use core::{ cell::UnsafeCell, fmt, str::from_utf8 };

pub trait DataLen {
    const LEN: usize;
}

pub trait Initialized {
    fn is_initialized(&self) -> bool;
}

#[inline(always)]
pub unsafe fn load_acc<T: DataLen + Initialized>(bytes: &[u8]) -> Result<&T, ProgramError> {
    load_acc_unchecked::<T>(bytes).and_then(|acc| {
        if acc.is_initialized() { Ok(acc) } else { Err(ProgramError::UninitializedAccount) }
    })
}

#[inline(always)]
pub unsafe fn load_acc_unchecked<T: DataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

#[inline(always)]
pub unsafe fn load_acc_mut<T: DataLen + Initialized>(
    bytes: &mut [u8]
) -> Result<&mut T, ProgramError> {
    load_acc_mut_unchecked::<T>(bytes).and_then(|acc| {
        if acc.is_initialized() { Ok(acc) } else { Err(ProgramError::UninitializedAccount) }
    })
}

#[inline(always)]
pub unsafe fn load_acc_mut_unchecked<T: DataLen>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&mut *(bytes.as_mut_ptr() as *mut T))
}

#[inline(always)]
pub unsafe fn load_ix_data<T: DataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidInstructionData.into());
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

pub unsafe fn to_bytes<T: DataLen>(data: &T) -> &[u8] {
    core::slice::from_raw_parts(data as *const T as *const u8, T::LEN)
}

pub unsafe fn to_mut_bytes<T: DataLen>(data: &mut T) -> &mut [u8] {
    core::slice::from_raw_parts_mut(data as *mut T as *mut u8, T::LEN)
}

//---------- Stake Program Utils -------------

pub fn collect_signers(
    accounts: &[AccountInfo],
    signers_arr: &mut [Pubkey; MAX_SIGNERS]
) -> Result<usize, ProgramError> {
    let mut signer_len = 0;

    for account in accounts {
        if account.is_signer() {
            if signer_len >= MAX_SIGNERS {
                return Err(ProgramError::AccountDataTooSmall);
            }
            signers_arr[signer_len] = *account.key();
            signer_len += 1;
        }
    }

    Ok(signer_len)
}

pub fn next_account_info<'a, I: Iterator<Item = &'a AccountInfo>>(
    iter: &mut I
) -> Result<&'a AccountInfo, ProgramError> {
    iter.next().ok_or(ProgramError::NotEnoughAccountKeys)
}

#[macro_export]
macro_rules! impl_sysvar_id {
    ($type:ty) => {
        impl $crate::state::stake_history::SysvarId for $type {
            fn id() -> Pubkey {
                id()
            }

            fn check_id(pubkey: &Pubkey) -> bool {
                check_id(pubkey)
            }
        }
    };
}

#[macro_export]
macro_rules! declare_sysvar_id {
    ($name:expr, $type:ty) => (
        pinocchio_pubkey::declare_id!($name);
        $crate::impl_sysvar_id!($type);
    );
}

/// After calling `validate_split_amount()`, this struct contains calculated
/// values that are used by the caller.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct ValidatedSplitInfo {
    pub source_remaining_balance: u64,
    pub destination_rent_exempt_reserve: u64,
}

/// Ensure the split amount is valid.  This checks the source and destination
/// accounts meet the minimum balance requirements, which is the rent exempt
/// reserve plus the minimum stake delegation, and that the source account has
/// enough lamports for the request split amount.  If not, return an error.
pub(crate) fn validate_split_amount(
    source_lamports: u64,
    destination_lamports: u64,
    split_lamports: u64,
    source_meta: &Meta,
    destination_data_len: usize,
    additional_required_lamports: u64,
    source_is_active: bool
) -> Result<ValidatedSplitInfo, ProgramError> {
    // Split amount has to be something
    if split_lamports == 0 {
        return Err(ProgramError::InsufficientFunds);
    }

    // Obviously cannot split more than what the source account has
    if split_lamports > source_lamports {
        return Err(ProgramError::InsufficientFunds);
    }

    // Verify that the source account still has enough lamports left after
    // splitting: EITHER at least the minimum balance, OR zero (in this case the
    // source account is transferring all lamports to new destination account,
    // and the source account will be closed)
    let source_minimum_balance = u64
        ::from_le_bytes(source_meta.rent_exempt_reserve)
        .saturating_add(additional_required_lamports);
    let source_remaining_balance = source_lamports.saturating_sub(split_lamports);
    if source_remaining_balance == 0 {
        // full amount is a withdrawal
        // nothing to do here
    } else if source_remaining_balance < source_minimum_balance {
        // the remaining balance is too low to do the split
        return Err(ProgramError::InsufficientFunds);
    } else {
        // all clear!
        // nothing to do here
    }

    let rent = Rent::get()?;
    let destination_rent_exempt_reserve = rent.minimum_balance(destination_data_len);

    // If the source is active stake, one of these criteria must be met:
    // 1. the destination account must be prefunded with at least the rent-exempt
    //    reserve, or
    // 2. the split must consume 100% of the source
    if
        source_is_active &&
        source_remaining_balance != 0 &&
        destination_lamports < destination_rent_exempt_reserve
    {
        return Err(ProgramError::InsufficientFunds);
    }

    // Verify the destination account meets the minimum balance requirements
    // This must handle:
    // 1. The destination account having a different rent exempt reserve due to data
    //    size changes
    // 2. The destination account being prefunded, which would lower the minimum
    //    split amount
    let destination_minimum_balance = destination_rent_exempt_reserve.saturating_add(
        additional_required_lamports
    );
    let destination_balance_deficit =
        destination_minimum_balance.saturating_sub(destination_lamports);
    if split_lamports < destination_balance_deficit {
        return Err(ProgramError::InsufficientFunds);
    }

    Ok(ValidatedSplitInfo {
        source_remaining_balance,
        destination_rent_exempt_reserve,
    })
}

//-------------- Solana Program Sysvar Copies ---------------

//---------------- This Get Sysvar was assisted by AI, needs to be checked ----------------------
//For this syscall mock, unlike solana program we use single thread to mantain the no_std enviorement
//Defining a generic Lazy<T> struct with interior mutability
pub struct Lazy<T> {
    value: UnsafeCell<Option<T>>,
}

impl<T> Lazy<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        // SAFETY: Only safe because Solana programs are single-threaded.
        // So its ok to get mutable access (even though `self` is shared!)
        unsafe {
            let value = &mut *self.value.get();
            if value.is_none() {
                *value = Some(init());
            }
            value.as_ref().unwrap()
        }
    }
}

static SYSCALL_STUBS: Lazy<Box<dyn SyscallStubs>> = Lazy::new();

unsafe impl<T> Sync for Lazy<T> {} //although this is telling that is available for multithreading, we know it wont happen

/// Builtin return values occupy the upper 32 bits
const BUILTIN_BIT_SHIFT: usize = 32;
macro_rules! to_builtin {
    ($error:expr) => {
        ($error as u64) << BUILTIN_BIT_SHIFT
    };
}

pub const UNSUPPORTED_SYSVAR: u64 = to_builtin!(17);

pub trait SyscallStubs: Sync + Send {
    fn sol_get_sysvar(
        &self,
        _sysvar_id_addr: *const u8,
        _var_addr: *mut u8,
        _offset: u64,
        _length: u64
    ) -> u64 {
        UNSUPPORTED_SYSVAR
    }
}

pub struct DefaultSyscallStubs {}

impl SyscallStubs for DefaultSyscallStubs {}

#[allow(dead_code)]
pub(crate) fn sol_get_sysvar(
    sysvar_id_addr: *const u8,
    var_addr: *mut u8,
    offset: u64,
    length: u64
) -> u64 {
    SYSCALL_STUBS.get_or_init(|| Box::new(DefaultSyscallStubs {})).sol_get_sysvar(
        sysvar_id_addr,
        var_addr,
        offset,
        length
    )
}

//---------------- End of AI assistance ----------------------

/// Handler for retrieving a slice of sysvar data from the `sol_get_sysvar`
/// syscall.
pub fn get_sysvar(
    dst: &mut [u8],
    sysvar_id: &Pubkey,
    offset: u64,
    length: u64
) -> Result<(), ProgramError> {
    // Check that the provided destination buffer is large enough to hold the
    // requested data.
    if dst.len() < (length as usize) {
        return Err(ProgramError::InvalidArgument);
    }

    let sysvar_id = sysvar_id as *const _ as *const u8;
    let var_addr = dst as *mut _ as *mut u8;

    //if on Solana call the actual syscall
    #[cfg(target_os = "solana")]
    let result = unsafe {
        pinocchio::syscalls::sol_get_sysvar(sysvar_id, var_addr, offset, length)
    };

    //if not on chain use the mock
    #[cfg(not(target_os = "solana"))]
    let result = sol_get_sysvar(sysvar_id, var_addr, offset, length);

    match result {
        SUCCESS => Ok(()),
        e => Err(e.into()),
    }
}

pub fn to_program_error(e: ProgramError) -> ProgramError {
    ProgramError::try_from(e).unwrap_or(ProgramError::InvalidAccountData)
}

#[inline(always)]
pub fn get_minimum_delegation() -> u64 {
    if FEATURE_STAKE_RAISE_MINIMUM_DELEGATION_TO_1_SOL {
        const MINIMUM_DELEGATION_SOL: u64 = 1;
        MINIMUM_DELEGATION_SOL * LAMPORTS_PER_SOL
    } else {
        1
    }
}

pub fn do_authorize(
    stake_account_info: &AccountInfo,
    signers: &[Pubkey],
    new_authority: &Pubkey,
    authority_type: StakeAuthorize,
    custodian: Option<&Pubkey>,
    clock: Clock,
) -> ProgramResult {
    let mut stake_account: pinocchio::account_info::RefMut<'_, StakeStateV2> =
        try_get_stake_state_mut(stake_account_info)?;
    match *stake_account {
        StakeStateV2::Initialized(mut meta) => {
            meta.authorized
                .authorize(
                    signers,
                    new_authority,
                    authority_type,
                    Some((&meta.lockup, &clock, custodian)),
                )
                .map_err(to_program_error)?;
            *stake_account = StakeStateV2::Initialized(meta);
            Ok(())
        }
        StakeStateV2::Stake(mut meta, stake, stake_flags) => {
            meta.authorized
                .authorize(
                    signers,
                    new_authority,
                    authority_type,
                    Some((&meta.lockup, &clock, custodian)),
                )
                .map_err(to_program_error)?;

            *stake_account = StakeStateV2::Stake(meta, stake, stake_flags);
            Ok(())
        }
        _ => Err(ProgramError::InvalidAccountData),
    }
}

// Means that no more than RATE of current effective stake may be added or subtracted per
// epoch.

pub fn warmup_cooldown_rate(
    current_epoch: [u8; 8],
    new_rate_activation_epoch: Option<[u8; 8]>
) -> f64 {
    let current = bytes_to_u64(current_epoch);
    let activation = new_rate_activation_epoch.map(bytes_to_u64).unwrap_or(u64::MAX);

    if current < activation {
        DEFAULT_WARMUP_COOLDOWN_RATE
    } else {
        NEW_WARMUP_COOLDOWN_RATE
    }
}

pub fn add_le_bytes(lhs: [u8; 8], rhs: [u8; 8]) -> [u8; 8] {
    u64::from_le_bytes(lhs).saturating_add(u64::from_le_bytes(rhs)).to_le_bytes()
}

pub fn bytes_to_u64(bytes: [u8; 8]) -> u64 {
    u64::from_le_bytes(bytes)
}

// MoveStake, MoveLamports, Withdraw, and AuthorizeWithSeed assemble signers explicitly
pub fn collect_signers_checked<'a>(
    authority_info: Option<&'a AccountInfo>,
    custodian_info: Option<&'a AccountInfo>,
) -> Result<([Pubkey; MAX_SIGNERS], Option<&'a Pubkey>, usize), ProgramError> {
    let mut signers: [Pubkey; MAX_SIGNERS] = Default::default();
    let mut signers_count = 0;

    if let Some(authority_info) = authority_info {
        add_signer(&mut signers, &mut signers_count, authority_info)?;
    }

    let custodian = if let Some(custodian_info) = custodian_info {
        add_signer(&mut signers, &mut signers_count, custodian_info)?;
        Some(custodian_info.key())
    } else {
        None
    };

    Ok((signers, custodian, signers_count))
}

pub fn add_signer(
    signers: &mut [Pubkey; MAX_SIGNERS],
    signers_count: &mut usize,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    if *signers_count >= MAX_SIGNERS {
        return Err(ProgramError::MaxAccountsDataAllocationsExceeded);
    }
    if !account_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    signers[*signers_count] = *account_info.key();
    *signers_count += 1;
    Ok(())
}

pub fn move_stake_or_lamports_shared_checks(
    source_stake_account_info: &AccountInfo,
    destination_stake_account_info: &AccountInfo,
    stake_authority_info: &AccountInfo,
) -> Result<(MergeKind, MergeKind), ProgramError> {
    // authority must sign
    let (signers, _, _) = collect_signers_checked(Some(stake_authority_info), None)?;

    // confirm not the same account
    if *source_stake_account_info.key() == *destination_stake_account_info.key() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // source and destination must be writable
    // runtime guards against unowned writes, but MoveStake and MoveLamports are defined by SIMD
    // we check explicitly to avoid any possibility of a successful no-op that never attempts to write
    if !source_stake_account_info.is_writable() || !destination_stake_account_info.is_writable() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let clock = Clock::get()?;
    let stake_history = StakeHistorySysvar(clock.epoch);

    // get_if_mergeable ensures accounts are not partly activated or in any form of deactivating
    // we still need to exclude activating state ourselves
    let source_merge_kind = MergeKind::get_if_mergeable(
        &*get_stake_state(source_stake_account_info)?,
        source_stake_account_info.lamports(),
        &clock,
        &stake_history,
    )?;

    // Authorized staker is allowed to move stake
    source_merge_kind
        .meta()
        .authorized
        .check(&signers, StakeAuthorize::Staker)
        .map_err(to_program_error)?;

    // same transient assurance as with source
    let destination_merge_kind = MergeKind::get_if_mergeable(
        &*get_stake_state(destination_stake_account_info)?,
        destination_stake_account_info.lamports(),
        &clock,
        &stake_history,
    )?;

    // ensure all authorities match and lockups match if lockup is in force
    MergeKind::metas_can_merge(
        source_merge_kind.meta(),
        destination_merge_kind.meta(),
        &clock,
    )?;

    Ok((source_merge_kind, destination_merge_kind))
}

//from_account_info helper for Clock while not implemente by Pinocchio
pub fn clock_from_account_info(account_info: &AccountInfo) -> Result<Ref<Clock>, ProgramError> {
    if account_info.data_len() != core::mem::size_of::<Clock>() {
        return Err(ProgramError::InvalidAccountData);
    }

    if account_info.key() != &CLOCK_ID {
        return Err(ProgramError::InvalidAccountData);
    }

    let data = account_info.try_borrow_data()?;

    Ok(Ref::map(data, |data| unsafe {
        &*(data.as_ptr() as *const Clock)
    }))
}

/// After calling `validate_delegated_amount()`, this struct contains calculated
/// values that are used by the caller.
pub(crate) struct ValidatedDelegatedInfo {
    pub stake_amount: [u8; 8],
}

pub(crate) fn new_stake(
    stake: [u8; 8],
    voter_pubkey: &Pubkey,
    vote_state: &VoteState,
    activation_epoch: [u8; 8]
) -> Stake {
    Stake {
        delegation: Delegation::new(
            voter_pubkey,
            bytes_to_u64(stake),
            activation_epoch
        ),
        credits_observed: vote_state.credits().to_le_bytes(),
    }
}

/// Ensure the stake delegation amount is valid.  This checks that the account
/// meets the minimum balance requirements of delegated stake.  If not, return
/// an error.
pub(crate) fn validate_delegated_amount(
    account: &AccountInfo,
    meta: &Meta
) -> Result<ValidatedDelegatedInfo, ProgramError> {
    let stake_amount = account.lamports().saturating_sub(bytes_to_u64(meta.rent_exempt_reserve)); // can't stake the rent

    // Stake accounts may be initialized with a stake amount below the minimum
    // delegation so check that the minimum is met before delegation.
    if stake_amount < get_minimum_delegation() {
        return Err(StakeError::InsufficientDelegation.into());
    }
    Ok(ValidatedDelegatedInfo { stake_amount: stake_amount.to_be_bytes() })
}

pub(crate) fn redelegate_stake(
    stake: &mut Stake,
    stake_lamports: [u8; 8],
    voter_pubkey: &Pubkey,
    vote_state: &VoteState,
    epoch: [u8;8],
    stake_history: &StakeHistorySysvar
) -> Result<(), ProgramError> {
    // If stake is currently active:
    if
        stake.stake(epoch, stake_history, PERPETUAL_NEW_WARMUP_COOLDOWN_RATE_EPOCH) !=
        0
    {
        // If pubkey of new voter is the same as current,
        // and we are scheduled to start deactivating this epoch,
        // we rescind deactivation
        if
            stake.delegation.voter_pubkey == *voter_pubkey &&
            epoch == stake.delegation.deactivation_epoch
        {
            stake.delegation.deactivation_epoch = u64::MAX.to_le_bytes();
            return Ok(());
        } else {
            // can't redelegate to another pubkey if stake is active.
            return Err(StakeError::TooSoonToRedelegate.into());
        }
    }
    // Either the stake is freshly activated, is active but has been
    // deactivated this epoch, or has fully de-activated.
    // Redelegation implies either re-activation or un-deactivation

    stake.delegation.stake = stake_lamports;
    stake.delegation.activation_epoch = epoch;
    stake.delegation.deactivation_epoch = u64::MAX.to_le_bytes();
    stake.delegation.voter_pubkey = *voter_pubkey;
    stake.credits_observed = vote_state.credits().to_be_bytes();
    Ok(())
}

// --- Hash struct and impls ----

#[cfg_attr(feature = "bytemuck", derive(Pod, Zeroable))]
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Hash(pub(crate) [u8; HASH_BYTES]);

impl From<[u8; HASH_BYTES]> for Hash {
    fn from(from: [u8; 32]) -> Self {
        Self(from)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

fn write_as_base58(f: &mut fmt::Formatter, h: &Hash) -> fmt::Result {
    let mut out = [0u8; MAX_BASE58_LEN];
    let out_slice: &mut [u8] = &mut out;
    // This will never fail because the only possible error is BufferTooSmall,
    // and we will never call it with too small a buffer.
    let len = bs58::encode(h.0).onto(out_slice).unwrap();
    let as_str = from_utf8(&out[..len]).unwrap();
    f.write_str(as_str)
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_as_base58(f, self)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_as_base58(f, self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseHashError {
    WrongSize,
    Invalid,
}

// #[cfg(feature = "std")]
// impl std::error::Error for ParseHashError {}

impl fmt::Display for ParseHashError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseHashError::WrongSize => f.write_str("string decoded to wrong size for hash"),
            ParseHashError::Invalid => f.write_str("failed to decoded string to hash"),
        }
    }
}

// requires the solana_sdk::bs58 crate
// impl FromStr for Hash {
//     type Err = ParseHashError;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         if s.len() > MAX_BASE58_LEN {
//             return Err(ParseHashError::WrongSize);
//         }
//         let mut bytes = [0; HASH_BYTES];
//         let decoded_size = bs58::decode(s)
//             .onto(&mut bytes)
//             .map_err(|_| ParseHashError::Invalid)?;
//         if decoded_size != mem::size_of::<Hash>() {
//             Err(ParseHashError::WrongSize)
//         } else {
//             Ok(bytes.into())
//         }
//     }
// }

impl Hash {
    #[deprecated(since = "2.2.0", note = "Use 'Hash::new_from_array' instead")]
    pub fn new(hash_slice: &[u8]) -> Self {
        Hash(<[u8; HASH_BYTES]>::try_from(hash_slice).unwrap())
    }

    pub const fn new_from_array(hash_array: [u8; HASH_BYTES]) -> Self {
        Self(hash_array)
    }

    // /// unique Hash for tests and benchmarks.
    // pub fn new_unique() -> Self {
    //     use solana_atomic_u64::AtomicU64;
    //     static I: AtomicU64 = AtomicU64::new(1);

    //     let mut b = [0u8; HASH_BYTES];
    //     let i = I.fetch_add(1);
    //     b[0..8].copy_from_slice(&i.to_le_bytes());
    //     Self::new_from_array(b)
    // }

    // pub fn to_bytes(self) -> [u8; HASH_BYTES] {
    //     self.0
    // }
}

#[cfg(target_arch = "wasm32")]
#[allow(non_snake_case)]
#[wasm_bindgen]
impl Hash {
    /// Create a new Hash object
    ///
    /// * `value` - optional hash as a base58 encoded string, `Uint8Array`, `[number]`
    #[wasm_bindgen(constructor)]
    pub fn constructor(value: JsValue) -> Result<Hash, JsValue> {
        if let Some(base58_str) = value.as_string() {
            base58_str.parse::<Hash>().map_err(|x| JsValue::from(x.to_string()))
        } else if let Some(uint8_array) = value.dyn_ref::<Uint8Array>() {
            <[u8; HASH_BYTES]>
                ::try_from(uint8_array.to_vec())
                .map(Hash::new_from_array)
                .map_err(|err| format!("Invalid Hash value: {err:?}").into())
        } else if let Some(array) = value.dyn_ref::<Array>() {
            let mut bytes = vec![];
            let iterator = js_sys::try_iter(&array.values())?.expect("array to be iterable");
            for x in iterator {
                let x = x?;

                if let Some(n) = x.as_f64() {
                    if n >= 0.0 && n <= 255.0 {
                        bytes.push(n as u8);
                        continue;
                    }
                }
                return Err(format!("Invalid array argument: {:?}", x).into());
            }
            <[u8; HASH_BYTES]>
                ::try_from(bytes)
                .map(Hash::new_from_array)
                .map_err(|err| format!("Invalid Hash value: {err:?}").into())
        } else if value.is_undefined() {
            Ok(Hash::default())
        } else {
            Err("Unsupported argument".into())
        }
    }

    /// Return the base58 string representation of the hash
    pub fn toString(&self) -> String {
        self.to_string()
    }

    /// Checks if two `Hash`s are equal
    pub fn equals(&self, other: &Hash) -> bool {
        self == other
    }

    /// Return the `Uint8Array` representation of the hash
    pub fn toBytes(&self) -> Box<[u8]> {
        self.0.clone().into()
    }
}