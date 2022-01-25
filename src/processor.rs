use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

use spl_token::state::Account as TokenAccount;
use spl_token::state::Mint as Mint;


use pyth_client::{AccountType, CorpAction, Mapping, Price, PriceStatus, PriceType, Product, cast};
use bytemuck::{cast_slice_mut, from_bytes_mut, try_cast_slice_mut};
use std::cell::RefMut;
solana_program::declare_id!("BpfProgram1111111111111111111111111111111111");

use crate::{error::OracleError, instruction::OracleInstruction, states::Oracle, states::AMM};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
    let instruction = OracleInstruction::unpack(instruction_data)?;

        match instruction {
            OracleInstruction::InitOracle { amm_program_id, lpcp_program_id, ucp_program_id } => {
                msg!("Instruction: InitOracle");
                Self::process_init_oracle(accounts, amm_program_id, lpcp_program_id, ucp_program_id, program_id)
            }
            OracleInstruction::TradeUSDI { amount } => {
                msg!("Instruction: TradeUSDI");
                Self::process_trade_usdi(accounts, amount, program_id)
            }
            OracleInstruction::TradeiAsset { amount } => {
                msg!("Instruction: TradeiAsset");
                Self::process_trade_iasset(accounts, amount, program_id)
            }
            OracleInstruction::CollateralCorrection { num_amms } => {
                msg!("Instruction: CollateralCorrection");

                // Collect oracle price
                
                let account_info_iter = &mut accounts.iter().peekable();
                let pyth_product_info = next_account_info(account_info_iter)?;
                let pyth_price_info = next_account_info(account_info_iter)?;

                let pyth_product_data = &pyth_product_info.try_borrow_data()?;
                let pyth_product = pyth_client::cast::<pyth_client::Product>(pyth_product_data);

                //Checks for pyth magic number
                if pyth_product.magic != pyth_client::MAGIC {
                    msg!("Pyth product account provided is not valid Pyth acccount");
                    return Err(ProgramError::InvalidArgument.into());
                }
                if pyth_product.atype != pyth_client::AccountType::Product as u32 {
                    msg!("Pyth product account provided is not a valid Pyth product account");
                    return Err(ProgramError::InvalidArgument.into());
                }

                if pyth_product.ver != pyth_client::VERSION_2 {
                    msg!("Pyth product account provided has a different version than the Pyth client");
                    return Err(ProgramError::InvalidArgument.into());
                }

                if !pyth_product.px_acc.is_valid() {
                    msg!("Pyth product price account is invalid");
                    return Err(ProgramError::InvalidArgument.into());
                }

                let pyth_price_pubkey = Pubkey::new(&pyth_product.px_acc.val);
                if &pyth_price_pubkey != pyth_price_info.key {
                    msg!("Pyth product price account does not match the Pyth price provided");
                    return Err(ProgramError::InvalidArgument.into());
                }

                let pyth_price_data = &pyth_price_info.try_borrow_data()?;
                let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);
                //oracle_price is in f64
                let exponent = pyth_price.expo;
                let base_oracle_price = pyth_price.agg.price as f64;
                let oracle_price = base_oracle_price.powi(exponent);

                msg!(" price account .. {:?}", pyth_price_info.key);
                msg!(" price type ... {}", get_price_type(&pyth_price.ptype));
                msg!(" status .... {}", get_status(&pyth_price.agg.status));
                msg!(" price ....{}", oracle_price);

                Self::process_collateral_correction(accounts, oracle_price, num_amms, program_id)
            }
        }
    }

    fn process_init_oracle(
        accounts: &[AccountInfo],
        amm_program_id: Pubkey,
        lpcp_program_id: Pubkey,
        ucp_program_id: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();


        //Ensure initializer signs off on instruction

        let initializer_account = next_account_info(account_info_iter)?;
        if !initializer_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }



        //Ensures Oracle account is rent exempt and will not be terminated

        let temp_fee_token_account = next_account_info(account_info_iter)?;

        let oracle_account = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        if !rent.is_exempt(oracle_account.lamports(), oracle_account.data_len()) {
            return Err(OracleError::NotRentExempt.into());
        }



        //Packs ucp state into account

        let mut oracle_info = Oracle::unpack_unchecked(&oracle_account.data.borrow())?;
        if oracle_info.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        oracle_info.is_initialized = true;
        oracle_info.fee_token_account_pubkey = *temp_fee_token_account.key;
        oracle_info.amm_program_id = amm_program_id;
        oracle_info.lpcp_program_id = lpcp_program_id;
        oracle_info.ucp_program_id = ucp_program_id;

        Oracle::pack(oracle_info, &mut oracle_account.data.borrow_mut())?;


        
        //Create and call instructions to initialize Oracle

        let (pda, _bump_seed) = Pubkey::find_program_address(&[b"incept"], program_id);

        let token_program = next_account_info(account_info_iter)?;
        let owner_change_ix = spl_token::instruction::set_authority(
            token_program.key,
            temp_fee_token_account.key,
            Some(&pda),
            spl_token::instruction::AuthorityType::AccountOwner,
            initializer_account.key,
            &[&initializer_account.key],
        )?;

        msg!("Calling the token program to transfer token account ownership...");
        invoke(
            &owner_change_ix,
            &[
                temp_fee_token_account.clone(),
                initializer_account.clone(),
                token_program.clone(),
            ],
        )?;

        Ok(())
    }

    fn process_trade_usdi(
        accounts: &[AccountInfo],
        iasset_amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();


        //Ensure correct account information

        let pda_fee_token_account = next_account_info(account_info_iter)?;

        let oracle_account = next_account_info(account_info_iter)?;
        let oracle_info = Oracle::unpack_unchecked(&oracle_account.data.borrow())?;
        if oracle_info.fee_token_account_pubkey != *pda_fee_token_account.key{
            return Err(ProgramError::InvalidAccountData);
        }

        let (pda, bump_seed) = Pubkey::find_program_address(&[b"incept"], program_id);

        let user_account = next_account_info(account_info_iter)?;
        let user_usdi_token_account = next_account_info(account_info_iter)?;
        let user_iasset_token_account = next_account_info(account_info_iter)?;
        let amm_pda_usdi_token_account = next_account_info(account_info_iter)?;
        let amm_pda_iasset_token_account = next_account_info(account_info_iter)?;

        let amm_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        let amm_pda_account = next_account_info(account_info_iter)?;




        //Set up program id, accounts, and instruction data to call TradeUSDI AMM instruction

        let amm_program_id = oracle_info.amm_program_id;

        let mut accounts = Vec::with_capacity(9);
        accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
        accounts.push(AccountMeta::new(*user_account.key, true));
        accounts.push(AccountMeta::new(*user_usdi_token_account.key, false)); 
        accounts.push(AccountMeta::new(*user_iasset_token_account.key, false));  
        accounts.push(AccountMeta::new(*amm_pda_usdi_token_account.key, false));  
        accounts.push(AccountMeta::new(*amm_pda_iasset_token_account.key, false));  
        accounts.push(AccountMeta::new(*amm_account.key, false));   
        accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
        accounts.push(AccountMeta::new_readonly(*amm_pda_account.key, false));

        let mut data: Vec<u8> = Vec::with_capacity(5);
        data.push(3);
        data.extend_from_slice(&iasset_amount.to_le_bytes());


        //Create and call instruction to trade USDI to the AMM

        let trade_usdi_to_amm = Instruction {
            program_id: amm_program_id,
            accounts,
            data,
        };
        
        invoke_signed(
            &trade_usdi_to_amm,
            &[
                pda_fee_token_account.clone(),
                user_account.clone(),
                user_usdi_token_account.clone(),
                user_iasset_token_account.clone(),
                amm_pda_usdi_token_account.clone(),
                amm_pda_iasset_token_account.clone(),
                amm_account.clone(),
                token_program.clone(),
                amm_pda_account.clone(),
            ],
            &[&[&b"incept"[..], &[bump_seed]]],
        )?;



        //Find spread picked up by last trade

        let amm_info = AMM::unpack_unchecked(&amm_account.data.borrow())?;
        let ucp_to_lpcp_spread_amount = amm_info.last_trade_spread;

        let ucp_account = next_account_info(account_info_iter)?;
        let ucp_collateral_token_account = next_account_info(account_info_iter)?;
        let lpcp_collateral_token_account = next_account_info(account_info_iter)?;
        let ucp_pda_account = next_account_info(account_info_iter)?;



        //Set up program id, accounts, and instruction data to call SendCollateralLPCP UCP instruction

        let ucp_program_id = oracle_info.ucp_program_id;

        let mut accounts = Vec::with_capacity(6);
        accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
        accounts.push(AccountMeta::new(*ucp_collateral_token_account.key, true));
        accounts.push(AccountMeta::new(*lpcp_collateral_token_account.key, false)); 
        accounts.push(AccountMeta::new(*ucp_account.key, false));  
        accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
        accounts.push(AccountMeta::new_readonly(*ucp_pda_account.key, false));

        let mut data: Vec<u8> = Vec::with_capacity(5);
        data.push(3);
        data.extend_from_slice(&ucp_to_lpcp_spread_amount.to_le_bytes());



        //Create and call instruction to send collateral from the UCP to the LPCP

        let send_collateral_to_lpcp = Instruction {
            program_id: ucp_program_id,
            accounts,
            data,
        };
        
        invoke_signed(
            &send_collateral_to_lpcp,
            &[
                pda_fee_token_account.clone(),
                ucp_collateral_token_account.clone(),
                lpcp_collateral_token_account.clone(),
                ucp_account.clone(),
                token_program.clone(),
                ucp_pda_account.clone(),
            ],
            &[&[&b"incept"[..], &[bump_seed]]],
        )?;

        Ok(())
    }

    fn process_trade_iasset(
        accounts: &[AccountInfo],
        iasset_amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();


        //Ensure correct account information

        let pda_fee_token_account = next_account_info(account_info_iter)?;

        let oracle_account = next_account_info(account_info_iter)?;
        let oracle_info = Oracle::unpack_unchecked(&oracle_account.data.borrow())?;
        if oracle_info.fee_token_account_pubkey != *pda_fee_token_account.key{
            return Err(ProgramError::InvalidAccountData);
        }

        let (pda, bump_seed) = Pubkey::find_program_address(&[b"incept"], program_id);

        let user_account = next_account_info(account_info_iter)?;
        let user_usdi_token_account = next_account_info(account_info_iter)?;
        let user_iasset_token_account = next_account_info(account_info_iter)?;
        let amm_pda_usdi_token_account = next_account_info(account_info_iter)?;
        let amm_pda_iasset_token_account = next_account_info(account_info_iter)?;

        let amm_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        let amm_pda_account = next_account_info(account_info_iter)?;




        //Set up program id, accounts, and instruction data to call TradeiAsset AMM instruction

        let amm_program_id = oracle_info.amm_program_id;

        let mut accounts = Vec::with_capacity(9);
        accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
        accounts.push(AccountMeta::new(*user_account.key, true));
        accounts.push(AccountMeta::new(*user_usdi_token_account.key, false)); 
        accounts.push(AccountMeta::new(*user_iasset_token_account.key, false));  
        accounts.push(AccountMeta::new(*amm_pda_usdi_token_account.key, false));  
        accounts.push(AccountMeta::new(*amm_pda_iasset_token_account.key, false));  
        accounts.push(AccountMeta::new(*amm_account.key, false));   
        accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
        accounts.push(AccountMeta::new_readonly(*amm_pda_account.key, false));

        let mut data: Vec<u8> = Vec::with_capacity(5);
        data.push(4);
        data.extend_from_slice(&iasset_amount.to_le_bytes());


        //Create and call instruction to trade iAsset to the AMM

        let trade_usdi_to_amm = Instruction {
            program_id: amm_program_id,
            accounts,
            data,
        };
        
        invoke_signed(
            &trade_usdi_to_amm,
            &[
                pda_fee_token_account.clone(),
                user_account.clone(),
                user_usdi_token_account.clone(),
                user_iasset_token_account.clone(),
                amm_pda_usdi_token_account.clone(),
                amm_pda_iasset_token_account.clone(),
                amm_account.clone(),
                token_program.clone(),
                amm_pda_account.clone(),
            ],
            &[&[&b"incept"[..], &[bump_seed]]],
        )?;



        //Find spread picked up by last trade

        let amm_info = AMM::unpack_unchecked(&amm_account.data.borrow())?;
        let ucp_to_lpcp_spread_amount = amm_info.last_trade_spread;

        let ucp_account = next_account_info(account_info_iter)?;
        let ucp_collateral_token_account = next_account_info(account_info_iter)?;
        let lpcp_collateral_token_account = next_account_info(account_info_iter)?;
        let ucp_pda_account = next_account_info(account_info_iter)?;



        //Set up program id, accounts, and instruction data to call SendCollateralLPCP UCP instruction

        let ucp_program_id = oracle_info.ucp_program_id;

        let mut accounts = Vec::with_capacity(6);
        accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
        accounts.push(AccountMeta::new(*ucp_collateral_token_account.key, true));
        accounts.push(AccountMeta::new(*lpcp_collateral_token_account.key, false)); 
        accounts.push(AccountMeta::new(*ucp_account.key, false));  
        accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
        accounts.push(AccountMeta::new_readonly(*ucp_pda_account.key, false));

        let mut data: Vec<u8> = Vec::with_capacity(5);
        data.push(3);
        data.extend_from_slice(&ucp_to_lpcp_spread_amount.to_le_bytes());



        //Create and call instruction to send collateral from the UCP to the LPCP

        let send_collateral = Instruction {
            program_id: ucp_program_id,
            accounts,
            data,
        };
        
        invoke_signed(
            &send_collateral,
            &[
                pda_fee_token_account.clone(),
                ucp_collateral_token_account.clone(),
                lpcp_collateral_token_account.clone(),
                ucp_account.clone(),
                token_program.clone(),
                ucp_pda_account.clone(),
            ],
            &[&[&b"incept"[..], &[bump_seed]]],
        )?;

        Ok(())
    }

    fn process_collateral_correction(
        accounts: &[AccountInfo],
        oracle_price: f64,
        num_amms: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();


        //Ensure correct account information

        let pda_fee_token_account = next_account_info(account_info_iter)?;
        
        let oracle_account = next_account_info(account_info_iter)?;
        let oracle_info = Oracle::unpack_unchecked(&oracle_account.data.borrow())?;
        if oracle_info.fee_token_account_pubkey != *pda_fee_token_account.key{
            return Err(ProgramError::InvalidAccountData);
        }



        //Set necessary account information

        let (pda, bump_seed) = Pubkey::find_program_address(&[b"incept"], program_id);

        let token_program = next_account_info(account_info_iter)?;
        let amm_pda_account = next_account_info(account_info_iter)?;


        let lpcp_collateral_token_account = next_account_info(account_info_iter)?;
        let ucp_collateral_token_account = next_account_info(account_info_iter)?;
        let lpcp_account = next_account_info(account_info_iter)?;
        let ucp_account = next_account_info(account_info_iter)?;
        let lpcp_pda_account = next_account_info(account_info_iter)?;
        let ucp_pda_account = next_account_info(account_info_iter)?;

        let mut transfer_amount = 0.0;


        // Loop through amms
        for _ in 0..num_amms{
            // Get amm information and insure valid data

            let current_amm = next_account_info(account_info_iter)?;
            let current_amm_info = AMM::unpack(&current_amm.data.borrow())?;
            let amm_usdi_token_account = next_account_info(account_info_iter)?;
            let amm_usdi_token_account_info = TokenAccount::unpack(&amm_usdi_token_account.data.borrow())?;
            let amm_iasset_token_account = next_account_info(account_info_iter)?;
            let amm_iasset_token_account_info = TokenAccount::unpack(&amm_iasset_token_account.data.borrow())?;
            let iasset_mint_account = next_account_info(account_info_iter)?;
            let iasset_mint_account_info = Mint::unpack(&iasset_mint_account.data.borrow())?;
            if iasset_mint_account.key != &amm_iasset_token_account_info.mint{
                return Err(OracleError::InvalidMintData.into());
            }
            if current_amm_info.usdi_token_account_pubkey != *amm_usdi_token_account.key || current_amm_info.iasset_token_account_pubkey != *amm_iasset_token_account.key{
                return Err(OracleError::MismatchedPDAAccountsForAMMs.into());
            }



            //Calculate number of iAsset to mint/burn

            let user_owned_iasset = iasset_mint_account_info.supply - amm_iasset_token_account_info.amount;
            let usdi_put_in = calc_sell_price_from_num_iasset(amm_usdi_token_account_info.amount, amm_iasset_token_account_info.amount, user_owned_iasset);
            let current_lpr_usdi = (amm_usdi_token_account_info.amount - usdi_put_in) as f64;
            let preferred_lpr = calc_lpr(oracle_price);
            let num_iasset_to_mint = (current_lpr_usdi - preferred_lpr*preferred_lpr)/preferred_lpr;

            if num_iasset_to_mint > 0.0{

                //Set up program id, accounts, and instruction data to call MintiAsset AMM instruction

                let amm_program_id = oracle_info.amm_program_id;

                let mut accounts = Vec::with_capacity(6);
                accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
                accounts.push(AccountMeta::new(*amm_iasset_token_account.key, true));
                accounts.push(AccountMeta::new(*iasset_mint_account.key, false)); 
                accounts.push(AccountMeta::new(*current_amm.key, false));  
                accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
                accounts.push(AccountMeta::new_readonly(*amm_pda_account.key, false));

                let mut data: Vec<u8> = Vec::with_capacity(5);
                data.push(5);
                let num_iasset_to_mint_u64 = to_u64(num_iasset_to_mint);
                data.extend_from_slice(&num_iasset_to_mint_u64.to_le_bytes());



                //Create and call instruction to mint iAsset to the AMM 

                let mint_iasset_to_amm = Instruction {
                    program_id: amm_program_id,
                    accounts,
                    data,
                };
        
                invoke_signed(
                    &mint_iasset_to_amm,
                    &[
                        pda_fee_token_account.clone(),
                        amm_iasset_token_account.clone(),
                        iasset_mint_account.clone(),
                        current_amm.clone(),
                        token_program.clone(),
                        amm_pda_account.clone(),
                    ],
                    &[&[&b"incept"[..], &[bump_seed]]],
                )?;
                
                
            } else {

                //Set up program id, accounts, and instruction to BurniAsset from the AMM 

                let amm_program_id = oracle_info.amm_program_id;

                let mut accounts = Vec::with_capacity(6);
                accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
                accounts.push(AccountMeta::new(*amm_iasset_token_account.key, true));
                accounts.push(AccountMeta::new(*iasset_mint_account.key, false)); 
                accounts.push(AccountMeta::new(*current_amm.key, false));  
                accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
                accounts.push(AccountMeta::new_readonly(*amm_pda_account.key, false));

                let mut data: Vec<u8> = Vec::with_capacity(5);
                data.push(6);
                let num_iasset_to_mint_u64 = to_u64(num_iasset_to_mint);
                data.extend_from_slice(&num_iasset_to_mint_u64.to_le_bytes());



                //Create and call instruction to burn iAsset from the AMM 

                let burn_iasset_from_amm = Instruction {
                    program_id: amm_program_id,
                    accounts,
                    data,
                };
        
                invoke_signed(
                    &burn_iasset_from_amm,
                    &[
                        pda_fee_token_account.clone(),
                        amm_iasset_token_account.clone(),
                        iasset_mint_account.clone(),
                        current_amm.clone(),
                        token_program.clone(),
                        amm_pda_account.clone(),
                    ],
                    &[&[&b"incept"[..], &[bump_seed]]],
                )?;
                
            }    

            //Recalculate LPR and add to total amount that will need to be transfered between UCP and LPCP

            let usdi_put_in_after_correction = calc_sell_price_from_num_iasset(amm_usdi_token_account_info.amount, amm_iasset_token_account_info.amount, user_owned_iasset); 
            let current_lpr_usdi_after_correction = (amm_usdi_token_account_info.amount - usdi_put_in_after_correction) as f64;  
            transfer_amount += current_lpr_usdi - current_lpr_usdi_after_correction;
        }

        if transfer_amount > 0.0{

            //Set up program id, accounts, and instruction data to call SendCollateralLPCP UCP instruction

            let ucp_program_id = oracle_info.ucp_program_id;

            let mut accounts = Vec::with_capacity(6);
            accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
            accounts.push(AccountMeta::new(*ucp_collateral_token_account.key, true));
            accounts.push(AccountMeta::new(*lpcp_collateral_token_account.key, false)); 
            accounts.push(AccountMeta::new(*ucp_account.key, false));  
            accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
            accounts.push(AccountMeta::new_readonly(*ucp_pda_account.key, false));

            let mut data: Vec<u8> = Vec::with_capacity(5);
            data.push(3);
            let transfer_amount_u64 = to_u64(transfer_amount);
            data.extend_from_slice(&transfer_amount_u64.to_le_bytes());



            //Create and call instruction to send collateral from the UCP to the LPCP

            let send_collateral_to_lpcp = Instruction {
                program_id: ucp_program_id,
                accounts,
                data,
            };
        
            invoke_signed(
                &send_collateral_to_lpcp,
                &[
                    pda_fee_token_account.clone(),
                    ucp_collateral_token_account.clone(),
                    lpcp_collateral_token_account.clone(),
                    ucp_account.clone(),
                    token_program.clone(),
                    ucp_pda_account.clone(),
                ],
                &[&[&b"incept"[..], &[bump_seed]]],
            )?;
            
            
        } else {

            //Set up program id, accounts, and instruction data to call SendCollateraUCP LPCP instruction

            let lpcp_program_id = oracle_info.lpcp_program_id;

            let mut accounts = Vec::with_capacity(6);
            accounts.push(AccountMeta::new(*pda_fee_token_account.key, true));
            accounts.push(AccountMeta::new(*lpcp_collateral_token_account.key, true));
            accounts.push(AccountMeta::new(*ucp_collateral_token_account.key, false)); 
            accounts.push(AccountMeta::new(*lpcp_account.key, false));  
            accounts.push(AccountMeta::new_readonly(*token_program.key, false)); 
            accounts.push(AccountMeta::new_readonly(*lpcp_pda_account.key, false));

            let mut data: Vec<u8> = Vec::with_capacity(5);
            data.push(3);
            let transfer_amount_u64 = to_u64(transfer_amount);
            data.extend_from_slice(&transfer_amount_u64.to_le_bytes());



            //Create and call instruction to send collateral from the UCP to the LPCP

            let send_collateral_to_lpcp = Instruction {
                program_id: lpcp_program_id,
                accounts,
                data,
            };
        
            invoke_signed(
                &send_collateral_to_lpcp,
                &[
                    pda_fee_token_account.clone(),
                    ucp_collateral_token_account.clone(),
                    lpcp_collateral_token_account.clone(),
                    ucp_account.clone(),
                    token_program.clone(),
                    ucp_pda_account.clone(),
                ],
                &[&[&b"incept"[..], &[bump_seed]]],
            )?;
        }   

        Ok(())
    }
}



fn get_price_type(ptype: &PriceType) -> &'static str {
    match ptype {
        PriceType::Unknown => "unknown",
        PriceType::Price => "price",
    }
}

fn get_status(st: &PriceStatus) -> &'static str {
    match st {
        PriceStatus::Unknown => "unknown",
        PriceStatus::Trading => "trading",
        PriceStatus::Halted => "halted",
        PriceStatus::Auction => "auction",
    }
}

fn get_corp_act(cact: &CorpAction) -> &'static str {
    match cact {
        CorpAction::NoCorpAct => "nocorpact",
    }
}

fn calc_sell_price_from_num_iasset(
    usdi_amm_amount: u64,
    iasset_amm_amount: u64,
    iasset_purchase_amount: u64,
) -> u64 {
    let mut delta_x = 0.0;
    let delta_y = 100.0;
    let mut x = usdi_amm_amount as f64;
    let mut y = iasset_amm_amount as f64;

    while (iasset_amm_amount as f64) - y > -(iasset_purchase_amount as f64) {
        delta_x = (2.0*delta_y*y*x + delta_y*delta_y*x)/(y*(2.0*y + 3.0*delta_y));
        x -= delta_x;
        y += delta_y;
    }
    return to_u64((usdi_amm_amount as f64) - x);
}

fn calc_lpr(
    oracle_price: f64,
) -> f64 {
    return oracle_price*0.9;
}

pub fn to_u64(value: f64) -> u64 {
    return value.abs().round() as u64;
}