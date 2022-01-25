// inside state.rs
use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

pub struct Oracle {
    pub is_initialized: bool,
    pub fee_token_account_pubkey: Pubkey,
    pub amm_program_id: Pubkey,
    pub lpcp_program_id: Pubkey,
    pub ucp_program_id: Pubkey,
}

impl Sealed for Oracle {}

impl IsInitialized for Oracle {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Oracle {
    const LEN: usize = 129;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Oracle::LEN];
        let (
            is_initialized,
            fee_token_account_pubkey,
            amm_program_id,
            lpcp_program_id,
            ucp_program_id,
        ) = array_refs![src, 1, 32, 32, 32, 32];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(Oracle {
            is_initialized, 
            fee_token_account_pubkey: Pubkey::new_from_array(*fee_token_account_pubkey),
            amm_program_id: Pubkey::new_from_array(*amm_program_id),
            lpcp_program_id: Pubkey::new_from_array(*lpcp_program_id),
            ucp_program_id: Pubkey::new_from_array(*ucp_program_id),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Oracle::LEN];
        let (
            is_initialized_dst,
            fee_token_account_pubkey_dst,
            amm_program_id_dst,
            lpcp_program_id_dst,
            ucp_program_id_dst,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 32];

        let Oracle {
            is_initialized,
            fee_token_account_pubkey,
            amm_program_id,
            lpcp_program_id,
            ucp_program_id,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        fee_token_account_pubkey_dst.copy_from_slice(fee_token_account_pubkey.as_ref());
        amm_program_id_dst.copy_from_slice(amm_program_id.as_ref());
        lpcp_program_id_dst.copy_from_slice(lpcp_program_id.as_ref());
        ucp_program_id_dst.copy_from_slice(ucp_program_id.as_ref());
    }
}


pub struct AMM {
    pub is_initialized: bool,
    pub usdi_token_account_pubkey: Pubkey,
    pub iasset_token_account_pubkey: Pubkey,
    pub oracle_pda_token_account_pubkey: Pubkey,
    pub lpcp_pda_token_account_pubkey: Pubkey,
    pub ucp_pda_token_account_pubkey: Pubkey,
    pub last_trade_spread: u64
}

impl Sealed for AMM {}

impl IsInitialized for AMM {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for AMM {
    const LEN: usize = 169;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, AMM::LEN];
        let (
            is_initialized,
            usdi_token_account_pubkey,
            iasset_token_account_pubkey,
            oracle_pda_token_account_pubkey,
            lpcp_pda_token_account_pubkey,
            ucp_pda_token_account_pubkey,
            last_trade_spread,
        ) = array_refs![src, 1, 32, 32, 32, 32, 32, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(AMM {
            is_initialized, 
            usdi_token_account_pubkey: Pubkey::new_from_array(*usdi_token_account_pubkey),
            iasset_token_account_pubkey: Pubkey::new_from_array(*iasset_token_account_pubkey),
            oracle_pda_token_account_pubkey: Pubkey::new_from_array(*oracle_pda_token_account_pubkey),
            lpcp_pda_token_account_pubkey: Pubkey::new_from_array(*lpcp_pda_token_account_pubkey),
            ucp_pda_token_account_pubkey: Pubkey::new_from_array(*ucp_pda_token_account_pubkey),
            last_trade_spread: u64::from_le_bytes(*last_trade_spread),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, AMM::LEN];
        let (
            is_initialized_dst,
            usdi_token_account_pubkey_dst,
            iasset_token_account_pubkey_dst,
            oracle_pda_token_account_pubkey_dst,
            lpcp_pda_token_account_pubkey_dst,
            ucp_pda_token_account_pubkey_dst,
            last_trade_spread_dst,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 32, 32, 8];

        let AMM {
            is_initialized,
            usdi_token_account_pubkey,
            iasset_token_account_pubkey,
            oracle_pda_token_account_pubkey,
            lpcp_pda_token_account_pubkey,
            ucp_pda_token_account_pubkey,
            last_trade_spread,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        usdi_token_account_pubkey_dst.copy_from_slice(usdi_token_account_pubkey.as_ref());
        iasset_token_account_pubkey_dst.copy_from_slice(iasset_token_account_pubkey.as_ref());
        oracle_pda_token_account_pubkey_dst.copy_from_slice(oracle_pda_token_account_pubkey.as_ref());
        lpcp_pda_token_account_pubkey_dst.copy_from_slice(lpcp_pda_token_account_pubkey.as_ref());
        ucp_pda_token_account_pubkey_dst.copy_from_slice(ucp_pda_token_account_pubkey.as_ref());
        *last_trade_spread_dst = last_trade_spread.to_le_bytes();
    }
}