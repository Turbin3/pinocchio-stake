pub mod my_state;
pub mod utils;

pub use my_state::*;
pub use utils::*;


#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, shank::ShankType)]
pub enum StakeAuthorize {
    Staker,
    Withdrawer,
}
