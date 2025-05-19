use anyhow::{anyhow, Result};
use arch_program::{
    account::AccountMeta, instruction::Instruction, pubkey::Pubkey,
    system_instruction::SystemInstruction,
};
use arch_sdk::helper::{
    assign_ownership_to_program, build_and_send_block, build_transaction,
    fetch_processed_transactions, generate_new_keypair, print_title,
};
use arch_sdk::processed_transaction::Status;
use arch_sdk::{
    constants::{NODE1_ADDRESS, PROGRAM_FILE_PATH},
    helper::{
        get_processed_transaction, read_account_info, send_utxo, sign_and_send_instruction,
        with_secret_key_file,
    },
};
use bitcoin::key::Keypair;
use borsh::BorshDeserialize;
use fungible_token_standard_program::{
    mint::{InitializeMintInput, TokenMintDetails},
    token_account::TokenBalance,
};

use crate::{
    instruction::initialize_mint_instruction,
    standard_tests::{MINT_FILE_PATH, MINT_OWNER_FILE_PATH},
};

pub const MINT_TEST_SUPPLY: u64 = 1000000u64;

pub const MINT_TEST_TICKER: &str = "ARCH";

pub const MINT_TEST_DECIMALS: u8 = 2;

pub const MINT_PRICE_SATS: u64 = 1000;

pub(crate) fn try_create_mint_account(single_use_mint: bool) -> Result<Pubkey> {
    println!();

    println!("\x1b[1m\x1b[32m===== MINT INITIALIZATION ===================================================================================================================================================================================\x1b[0m");

    let (mint_keypair, mint_pubkey) = match single_use_mint {
        false => with_secret_key_file(MINT_FILE_PATH).expect("getting caller info should not fail"),
        true => {
            let mint_account = generate_new_keypair();
            (mint_account.0, mint_account.1)
        }
    };

    let (program_keypair, program_pubkey) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (mint_owner_keypair, mint_owner_pubkey) =
        with_secret_key_file(MINT_OWNER_FILE_PATH).expect("getting caller info should not fail");

    if let Ok(account_info_result) = read_account_info(NODE1_ADDRESS, mint_pubkey) {
        match TokenMintDetails::try_from_slice(&account_info_result.data) {
            Ok(_mint_details) => {
                println!();
                println!("\x1b[33m Mint Details already exist in account ! Skipping mint initialization. \x1b[0m");
                return Ok(mint_pubkey);
            }
            Err(_) => {
                println!("Account exists but no mint details within !");
                panic!()
            }
        }
    };

    let (txid, vout) = send_utxo(mint_pubkey);

    println!(
        "\x1b[32m Step 1/3 Successful :\x1b[0m BTC Transaction for mint account UTXO successfully sent : {} -- vout : {}",
        arch_sdk::constants::get_explorer_tx_url(BITCOIN_NETWORK, &txid), vout
    );

    let account_creation_instruction = system_instruction::create_account(
        hex::decode(txid).unwrap().try_into().unwrap(),
        vout,
        mint_pubkey,
    );

    let ownership_transfer_instruction =
        system_instruction::assign_ownership(mint_pubkey, program_pubkey);

    let test_mint_input = InitializeMintInput::new(
        mint_owner_pubkey.serialize(),
        MINT_TEST_SUPPLY,
        MINT_TEST_TICKER.to_string(),
        MINT_TEST_DECIMALS,
    );

    let init_mint_instruction = initialize_mint_instruction(
        &mint_owner_pubkey,
        &mint_pubkey,
        &program_pubkey,
        test_mint_input,
    )
    .unwrap();

    let account_creation_transaction = build_transaction(
        vec![mint_owner_keypair],
        vec![account_creation_instruction, ownership_transfer_instruction],
    );

    let init_mint_transaction = build_transaction(vec![mint_keypair], vec![init_mint_instruction]);

    let block_transactions =
        build_and_send_block(vec![account_creation_transaction, init_mint_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    println!(
        "\x1b[32m Step 2/3 Successful :\x1b[0m Account creation and ownership assignment transaction succesfully processed !"
    );

    assert!(matches!(
        processed_transactions[1].status,
        Status::Processed
    ));

    let account_info = read_account_info(NODE1_ADDRESS, mint_pubkey).unwrap();

    let mint_details = TokenMintDetails::try_from_slice(&account_info.data).unwrap();

    assert!(account_info.owner == program_pubkey);

    assert!(!account_info.is_executable);

    println!("\x1b[32m Step 3/3 Successful :\x1b[0m Init Mint transaction succesfully processed !");

    println!("\x1b[1m\x1b[32m================================================================================== INITIALIZE  MINT : OK ! ==================================================================================\x1b[0m");

    Ok(mint_pubkey)
}

pub(crate) fn provide_empty_account_to_program(
    program_pubkey: &Pubkey,
) -> Result<(Keypair, Pubkey)> {
    let (account_key_pair, account_pubkey, address) = generate_new_keypair();

    let (txid, vout) = send_utxo(account_pubkey);

    println!(
        "\x1b[32m Step 1/3 Successful :\x1b[0m Account created with address  {}",
        address
    );

    let (txid, _) = sign_and_send_instruction(
        &system_instruction::create_account(
            hex::decode(txid).unwrap().try_into().unwrap(),
            vout,
            account_pubkey,
        ),
        vec![account_key_pair],
    )
    .expect("signing and sending a transaction should not fail");

    let _processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
        .expect("get processed transaction should not fail");

    assign_ownership_to_program(program_pubkey, account_pubkey, account_key_pair);

    Ok((account_key_pair, account_pubkey))
}

pub(crate) fn get_mint_info(account_pubkey: &Pubkey) -> Result<TokenMintDetails> {
    use borsh::BorshDeserialize;

    let account_info = read_account_info(NODE1_ADDRESS, *account_pubkey)
        .map_err(|e| anyhow!(format!("Error reading account content {}", e.to_string())))?;

    let mut account_info_data = account_info.data.as_slice();

    let account_counter = TokenMintDetails::deserialize(&mut account_info_data)
        .map_err(|e| anyhow!(format!("Error corrupted account data {}", e.to_string())))?;

    Ok(account_counter)
}

pub(crate) fn get_balance_account(account_pubkey: &Pubkey) -> Result<TokenBalance> {
    let account_info = read_account_info(NODE1_ADDRESS, *account_pubkey)
        .map_err(|e| anyhow!(format!("Error reading account content {}", e.to_string())))?;

    let mut account_info_data = account_info.data.as_slice();

    let token_balance = TokenBalance::deserialize(&mut account_info_data)
        .map_err(|e| anyhow!(format!("Error corrupted account data {}", e.to_string())))?;

    Ok(token_balance)
}

pub(crate) fn create_balance_account(
    account_pubkey: &Pubkey,
    account_keypair: Keypair,
    mint_pubkey: &Pubkey,
    token_program_pubkey: &Pubkey,
) -> Result<Pubkey> {
    print_title("Balance account creation", 1);

    let (balance_account_key_pair, balance_account_pubkey, balance_address) =
        generate_new_keypair();

    let (txid, vout) = send_utxo(balance_account_pubkey);

    let (txid, _) = sign_and_send_instruction(
        &system_instruction::create_account(
            hex::decode(txid).unwrap().try_into().unwrap(),
            vout,
            balance_account_pubkey,
        ),
        vec![balance_account_key_pair],
    )
    .expect("signing and sending a transaction should not fail");

    let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
        .expect("get processed transaction should not fail");

    assert!(matches!(processed_tx.status, Status::Processed));

    println!(
        "\x1b[32m Step 1/3 Successful :\x1b[0m Account created with address  {}",
        balance_address
    );

    assign_ownership_to_program(
        token_program_pubkey,
        balance_account_pubkey,
        balance_account_key_pair,
    );

    println!("\x1b[32m Step 2/3 Successful :\x1b[0m Account Ownership assigned to program ");

    let initialize_balance_account_instruction = Instruction {
        program_id: *token_program_pubkey,
        accounts: vec![
            AccountMeta {
                pubkey: *account_pubkey,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *mint_pubkey,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: balance_account_pubkey,
                is_signer: false,
                is_writable: true,
            },
        ],
        data: vec![1],
    };

    let account_initialization_transaction = build_transaction(
        vec![account_keypair],
        vec![initialize_balance_account_instruction],
    );
    let block_transactions = build_and_send_block(vec![account_initialization_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    println!("\x1b[32m Step 3/3 Successful :\x1b[0m Balance account successfully initialized");

    print_title("Balance account creation : OK !", 0);

    Ok(balance_account_pubkey)
}
