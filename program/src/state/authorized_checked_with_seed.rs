use alloc::vec::Vec;
use pinocchio::{program_error::ProgramError, pubkey::Pubkey};
use crate::state::stake_authorize::StakeAuthorize;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizeCheckedWithSeedArgs <'a>{
    pub stake_authorize: StakeAuthorize,
    pub authority_seed_len:u32,
    pub authority_seed: &'a str,
    pub authority_owner:Pubkey,
}

impl <'a> AuthorizeCheckedWithSeedArgs<'a>{
    pub fn serialize(&self)->Vec<u8> {
        
        //can just use Vec::new() 
        let mut buf= Vec::with_capacity(1+4+self.authority_seed.len() + 32);
        

        //serialize as a u8
        buf.push(self.stake_authorize as u8);

        //serialize the authority_seed_len
        buf.extend_from_slice(&(self.authority_seed_len).to_le_bytes());

        //serialize the authority seed
        buf.extend_from_slice(self.authority_seed.as_bytes());

        buf.extend_from_slice(self.authority_owner.as_ref());

        buf
        
    }

    fn deserialize(input: &'a [u8])->Result<Self, ProgramError>{
        if input.len() < 41{
            return Err(ProgramError::AccountDataTooSmall);
        }

        let mut offset=0;

        //deserialize StakeAuthorize
        let stake_authorize= match input.get(offset){
            Some(0)=>StakeAuthorize::Staker,
            Some(1)=>StakeAuthorize::Withdrawer,
            _ => return Err(ProgramError::InvalidInstructionData),
        };
        offset +=1;

        //deserialize authority_seed_len
        if input.len()< offset + 4{
         return Err(ProgramError::InvalidInstructionData);
        }
        let authority_seed_len= u32::from_le_bytes(input[offset..offset + 4].try_into().unwrap());
        offset +=4;

        let seed_len=authority_seed_len as usize;
        if input.len()< offset +seed_len{
            return Err(ProgramError::InvalidInstructionData)
        }

        let authority_seed=core::str::from_utf8(&input[offset..offset+seed_len]).map_err(|_| ProgramError::InvalidInstructionData)?;
        offset+=seed_len;

        if input.len() < offset + 32 {
            return Err(ProgramError::InvalidInstructionData);
        }
        
        let mut authority_owner = [0u8; 32];
        authority_owner.copy_from_slice(&input[offset..offset + 32]);

        offset +=32;
        
        Ok(Self{
            stake_authorize,
            authority_seed_len,
            authority_seed,
            authority_owner
        })

    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_serialize_deserialize() {
        // Create a sample instance
        let stake_authorize = StakeAuthorize::Staker;
        let authority_seed = "example_seed";
        let authority_owner: Pubkey = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 
            17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
        ];

        let args = AuthorizeCheckedWithSeedArgs {
            stake_authorize,
            authority_seed_len: authority_seed.len() as u32,
            authority_seed,
            authority_owner,
        };

        // Serialize the struct
        let serialized_data = args.serialize();
        
        // Deserialize it back
        let deserialized_args = AuthorizeCheckedWithSeedArgs::deserialize(&serialized_data)
            .expect("Deserialization should succeed");

        // Assertions
        assert_eq!(deserialized_args.stake_authorize, args.stake_authorize);
        assert_eq!(deserialized_args.authority_seed, args.authority_seed);
        assert_eq!(deserialized_args.authority_seed_len, args.authority_seed_len);
        assert_eq!(deserialized_args.authority_owner, args.authority_owner);
    }
}
