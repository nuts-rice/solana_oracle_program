use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey
};

use crate::processor::Processor;

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, instruction_data)
}


// //Tests for returning correct value
// #[cfg(test)]
// mod test {
//     use std::str::FromStr;

//     use solana_program::account_info::Account;
//     use solana_sdk::pubkey;

//     use {
//         super::*,
//         assert_matches::*,
//         solana_client::rpc_client::RpcClient,
//         solana_program::instruction::{AccountMeta, Instruction},
//         solana_program_test::*,
//         solana_sdk::{signature::Signer, transaction::Transaction},
//     };

//     #[tokio::test]
//     async fn test_transaction() {
//         let program_id = Pubkey::new_unique();
//         let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(

//             "Oracle_Program",
//             program_id,
//             processor!(process_instruction),
//         )
//         .start()
//         .await;

//         let mut transaction = Transaction::new_with_payer(
//             &[Instruction {
//                 program_id,
//                 accounts: vec![AccountMeta::new(payer.pubkey(), false)],
//                 data: vec![1, 2, 3],
//             }],
//             Some(&payer.pubkey()),
//         );
//         transaction.sign(&[&payer], recent_blockhash);

//         assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
//     }

//     //get mapping for eth info
//     //key 2ciUuGZiee5macAMeQ7bHGTJtwcYTgnt6jdmQnnKZrfu
//     #[test]
//     fn pull_price() {
//         let url = "http://api.devnet.solana.com";
//         let key = "BmA9Z6FjioHJPpjT39QazZyhDRUdZy2ezwx4GiDdE2u2";
//         let clnt = RpcClient::new(url.to_string());
//         let mut akey = Pubkey::from_str(key).unwrap();
//         let map_data = clnt.get_account_data(&akey).unwrap();
//         let map_acct = cast::<Mapping>(&map_data);
//         //Eth/usd product pyth client


//         let mut i = 0;
//         for prod_akey in &map_acct.products{
//             let prod_pkey = Pubkey::new(&prod_akey.val);
//             let prod_data = clnt.get_account_data(&prod_pkey).unwrap();
//             let prod_acct = cast::<Product>(&prod_data);
//         }
//     }
// }
