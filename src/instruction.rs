// inside instruction.rs
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::msg;
use std::convert::TryInto;
use crate::error::OracleError::InvalidInstruction;

pub enum OracleInstruction {

    /// Initializes the Oracle
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The Initializer Account
    /// 1. `[writable]` Temporary fee token account that should be created prior to this instruction and owned by The Initializer Account
    /// 2. `[writable]` The Oracle account holding the Oracle info
    /// 3. `[]` The rent sysvar
    /// 4. `[]` The token program
    InitOracle {
        amm_program_id: Pubkey,
        lpcp_program_id: Pubkey,
        ucp_program_id: Pubkey,
    },

    /// Allows user to trade {amount (in USDI)} of USDI in exchange for iAsset
    ///  
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Oracle token account that should be created prior to this instruction and owned by The Oracle Program
    /// 1. `[writable]` The Oracle account holding the Oracle info
    /// 2. `[signer]` The User's account
    /// 3. `[writable]` The USDI token account owned by the user's account
    /// 4. `[writable]` The iAsset token account owned by the user's account
    /// 5. `[writable]` The AMM PDA's USDI token account
    /// 6. `[writable]` The AMM PDA's iAsset token account
    /// 7. `[writable]` The AMM account holding the AMM info
    /// 8. `[]` The token program
    /// 9. `[]` The AMM PDA account
    /// 10. `[writable]` The UCP account holding the UCP info
    /// 11. `[signer]` UCP token account that should be created prior to this instruction and owned by The UCP Program
    /// 12. `[writable]` LPCP token account that should be created prior to this instruction and owned by The LPCP Program
    /// 13. `[]` The UCP PDA account
    TradeUSDI {
        amount: u64,
    },


    /// Allows user to trade {amount (in USDI)} of USDI in exchange for iAsset
    ///  
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Oracle token account that should be created prior to this instruction and owned by The Oracle Program
    /// 1. `[writable]` The Oracle account holding the Oracle info
    /// 2. `[signer]` The User's account
    /// 3. `[writable]` The USDI token account owned by the user's account
    /// 4. `[writable]` The iAsset token account owned by the user's account
    /// 5. `[writable]` The AMM PDA's USDI token account
    /// 6. `[writable]` The AMM PDA's iAsset token account
    /// 7. `[writable]` The AMM account holding the AMM info
    /// 8. `[]` The token program
    /// 9. `[]` The AMM PDA account
    /// 10. `[writable]` The UCP account holding the UCP info
    /// 11. `[signer]` UCP token account that should be created prior to this instruction and owned by The UCP Program
    /// 12. `[writable]` LPCP token account that should be created prior to this instruction and owned by The LPCP Program
    /// 13. `[]` The UCP PDA account
    TradeiAsset {
        amount: u64,
    },


    /// Allows user to withdraw USDI and claim collateral
    ///  
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Oracle token account that should be created prior to this instruction and owned by The Oracle Program
    /// 1. `[writable]` The Oracle account holding the Oracle info
    /// 2. `[]` The token program
    /// 3. `[]` The AMM PDA account
    /// 4. `[writable]` LPCP token account that should be created prior to this instruction and owned by The LPCP Program
    /// 5. `[signer]` UCP token account that should be created prior to this instruction and owned by The UCP Program
    /// 6. `[writable]` The LPCP account holding the LPCP info
    /// 7. `[writable]` The UCP account holding the UCP info
    /// 8. `[]` The LPCP PDA account
    /// 9. `[]` The UCP PDA account
    /// FOR EACH AMM IN INCEPT ECOSYSTEM
    /// 10 + 3i. `[writable]` The AMM account holding the AMM info
    /// 11 + 3i. `[writable]` The AMM PDA's USDI token account
    /// 12 + 3i. `[writable]` The AMM PDA's iAsset token account
    /// 13 + 3i. `[writable]` The AMM iAsset mint account
    CollateralCorrection {
        num_amms: u64,
    },
}

impl OracleInstruction {

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => Self::InitOracle {
                amm_program_id: Self::unpack_pubkey(rest, 0)?,
                lpcp_program_id: Self::unpack_pubkey(rest, 32)?,
                ucp_program_id: Self::unpack_pubkey(rest, 64)?,
            },
            1 => Self::TradeUSDI {
                amount: Self::unpack_amount(rest)?,
            },
            2 => Self::TradeiAsset {
                amount: Self::unpack_amount(rest)?,
            },
            3 => Self::CollateralCorrection {
                num_amms: Self::unpack_amount(rest)?,
            },
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_pubkey(input: &[u8], start: u8) -> Result<Pubkey, ProgramError> {
        let mut pubkey: Option<Pubkey> = None;
        let pubkey_array = input.get(usize::from(start)..usize::from(start)+32).and_then(|slice| slice.try_into().ok());
        match pubkey_array {
            Some(array) => matches!(Pubkey::new_from_array(array), pubkey),
            None => false,
        };
        match pubkey {
            Some(pubk) => return Ok(pubk),
            None => return Err(InvalidInstruction.into()),
        };

    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        let amount = input
            .get(0..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction)?;
        Ok(amount)
    }
}
