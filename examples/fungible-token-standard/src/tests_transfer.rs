use arch_sdk::helper::{
    build_and_send_block, build_transaction, fetch_processed_transactions, generate_new_keypair,
    try_deploy_program,
};
use arch_sdk::processed_transaction::Status;
use arch_sdk::{constants::PROGRAM_FILE_PATH, helper::init_logging};
use serial_test::serial;

use crate::{
    helpers::{
        create_balance_account, get_balance_account, get_mint_info, try_create_mint_account,
    },
    instruction::{mint_request_instruction, transfer_request_instruction},
    standard_tests::ELF_PATH,
};

#[ignore]
#[serial]
#[test]
fn mint_and_transfer_tokens() {
    init_logging();

    let mint_amount = 10u64;

    let program_pubkey =
        try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Fungible-Token-Standard").unwrap();

    let mint_account_pubkey = try_create_mint_account(true).unwrap();

    let previous_mint_details = get_mint_info(&mint_account_pubkey).unwrap();

    /* --------------------- CREATING FIRST BALANCE ACCOUNT --------------------- */
    let (first_account_owner_key_pair, first_account_owner_pubkey, _first_account_owner_address) =
        generate_new_keypair();

    let first_balance_account_pubkey = create_balance_account(
        &first_account_owner_pubkey,
        first_account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    /* --------------------- CREATING SECOND BALANCE ACCOUNT -------------------- */
    let (second_account_owner_key_pair, second_account_owner_pubkey, _second_account_owner_address) =
        generate_new_keypair();

    let second_balance_account_pubkey = create_balance_account(
        &second_account_owner_pubkey,
        second_account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    /* ------------------------ MINTING FOR FIRST ACCOUNT ----------------------- */
    let mint_instruction = mint_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &first_balance_account_pubkey,
        &first_account_owner_pubkey,
        mint_amount,
    )
    .unwrap();

    let mint_transaction =
        build_transaction(vec![first_account_owner_key_pair], vec![mint_instruction]);

    let block_transactions = build_and_send_block(vec![mint_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    let resulting_balance = get_balance_account(&first_balance_account_pubkey).unwrap();

    /* ------------------ TRANSFER FROM FIRST ACCOUNT TO SECOND ----------------- */
    let transfer_instruction = transfer_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &first_balance_account_pubkey,
        &first_account_owner_pubkey,
        &second_balance_account_pubkey,
        mint_amount,
    )
    .unwrap();

    let transfer_transaction = build_transaction(
        vec![first_account_owner_key_pair],
        vec![transfer_instruction],
    );

    let block_transactions = build_and_send_block(vec![transfer_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    let resulting_sender_balance = get_balance_account(&first_balance_account_pubkey).unwrap();

    let resulting_receiver_balance = get_balance_account(&second_balance_account_pubkey).unwrap();

    assert_eq!(resulting_receiver_balance.current_balance, mint_amount);

    assert_eq!(resulting_sender_balance.current_balance, 0);
}

#[ignore]
#[serial]
#[test]
fn mint_and_transfer_insufficient_balance() {
    init_logging();

    let mint_amount = 10u64;

    let program_pubkey =
        try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Fungible-Token-Standard").unwrap();

    let mint_account_pubkey = try_create_mint_account(true).unwrap();

    let previous_mint_details = get_mint_info(&mint_account_pubkey).unwrap();

    /* --------------------- CREATING FIRST BALANCE ACCOUNT --------------------- */
    let (first_account_owner_key_pair, first_account_owner_pubkey, _first_account_owner_address) =
        generate_new_keypair();

    let first_balance_account_pubkey = create_balance_account(
        &first_account_owner_pubkey,
        first_account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    /* --------------------- CREATING SECOND BALANCE ACCOUNT -------------------- */
    let (second_account_owner_key_pair, second_account_owner_pubkey, _second_account_owner_address) =
        generate_new_keypair();

    let second_balance_account_pubkey = create_balance_account(
        &second_account_owner_pubkey,
        second_account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    /* ------------------------ MINTING FOR FIRST ACCOUNT ----------------------- */
    let mint_instruction = mint_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &first_balance_account_pubkey,
        &first_account_owner_pubkey,
        mint_amount,
    )
    .unwrap();

    let mint_transaction =
        build_transaction(vec![first_account_owner_key_pair], vec![mint_instruction]);

    let block_transactions = build_and_send_block(vec![mint_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    let resulting_balance = get_balance_account(&first_balance_account_pubkey).unwrap();

    /* ------------------ TRANSFER FROM FIRST ACCOUNT TO SECOND ----------------- */
    let transfer_instruction = transfer_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &first_balance_account_pubkey,
        &first_account_owner_pubkey,
        &second_balance_account_pubkey,
        mint_amount * 2,
    )
    .unwrap();

    let transfer_transaction = build_transaction(
        vec![first_account_owner_key_pair],
        vec![transfer_instruction],
    );

    let block_transactions = build_and_send_block(vec![transfer_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    let resulting_sender_balance = get_balance_account(&first_balance_account_pubkey).unwrap();

    let resulting_receiver_balance = get_balance_account(&second_balance_account_pubkey).unwrap();

    assert_eq!(resulting_receiver_balance.current_balance, 0);

    assert_eq!(resulting_sender_balance.current_balance, mint_amount);
}

#[ignore]
#[serial]
#[test]
fn conescutive_transfers() {
    init_logging();

    let mint_amount = 10u64;

    let program_pubkey =
        try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Fungible-Token-Standard").unwrap();

    let mint_account_pubkey = try_create_mint_account(true).unwrap();

    let previous_mint_details = get_mint_info(&mint_account_pubkey).unwrap();

    /* --------------------- CREATING FIRST BALANCE ACCOUNT --------------------- */
    let (first_account_owner_key_pair, first_account_owner_pubkey, _first_account_owner_address) =
        generate_new_keypair();

    let first_balance_account_pubkey = create_balance_account(
        &first_account_owner_pubkey,
        first_account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    /* --------------------- CREATING SECOND BALANCE ACCOUNT -------------------- */
    let (second_account_owner_key_pair, second_account_owner_pubkey, _second_account_owner_address) =
        generate_new_keypair();

    let second_balance_account_pubkey = create_balance_account(
        &second_account_owner_pubkey,
        second_account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    /* ------------------------ MINTING FOR FIRST ACCOUNT ----------------------- */
    let mint_instruction = mint_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &first_balance_account_pubkey,
        &first_account_owner_pubkey,
        mint_amount,
    )
    .unwrap();

    let mint_transaction =
        build_transaction(vec![first_account_owner_key_pair], vec![mint_instruction]);

    let block_transactions = build_and_send_block(vec![mint_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    let resulting_balance = get_balance_account(&first_balance_account_pubkey).unwrap();

    /* --------------- FIRST TRANSFER FROM FIRST ACCOUNT TO SECOND -------------- */
    let transfer_instruction = transfer_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &first_balance_account_pubkey,
        &first_account_owner_pubkey,
        &second_balance_account_pubkey,
        mint_amount / 2,
    )
    .unwrap();

    let transfer_transaction = build_transaction(
        vec![first_account_owner_key_pair],
        vec![transfer_instruction],
    );

    let block_transactions = build_and_send_block(vec![transfer_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    let resulting_sender_balance = get_balance_account(&first_balance_account_pubkey).unwrap();

    let resulting_receiver_balance = get_balance_account(&second_balance_account_pubkey).unwrap();

    assert_eq!(resulting_receiver_balance.current_balance, mint_amount / 2);

    assert_eq!(resulting_sender_balance.current_balance, mint_amount / 2);

    /* -------------- SECOND TRANSFER FROM FIRST ACCOUNT TO SECOND -------------- */

    let transfer_instruction = transfer_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &first_balance_account_pubkey,
        &first_account_owner_pubkey,
        &second_balance_account_pubkey,
        mint_amount / 2,
    )
    .unwrap();

    let transfer_transaction = build_transaction(
        vec![first_account_owner_key_pair],
        vec![transfer_instruction],
    );

    let block_transactions = build_and_send_block(vec![transfer_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    let resulting_sender_balance = get_balance_account(&first_balance_account_pubkey).unwrap();

    let resulting_receiver_balance = get_balance_account(&second_balance_account_pubkey).unwrap();

    assert_eq!(resulting_receiver_balance.current_balance, mint_amount);

    assert_eq!(resulting_sender_balance.current_balance, 0);
}
