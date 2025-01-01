use arch_sdk::helper::{
    build_and_send_block, build_transaction, fetch_processed_transactions, generate_new_keypair,
    try_deploy_program,
};
use arch_sdk::processed_transaction::Status;
use arch_sdk::{constants::PROGRAM_FILE_PATH, helper::init_logging};
use fungible_token_standard_program::mint::MintStatus;
use serial_test::serial;

use crate::{
    helpers::{
        create_balance_account, get_balance_account, get_mint_info, try_create_mint_account,
        MINT_TEST_SUPPLY,
    },
    instruction::mint_request_instruction,
    standard_tests::ELF_PATH,
};

#[ignore]
#[serial]
#[test]
fn deploy_and_init_mint() {
    init_logging();

    let program_pubkey =
        try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Fungible-Token-Standard").unwrap();

    let mint_account_pubkey = try_create_mint_account(false).unwrap();
}

#[ignore]
#[serial]
#[test]
fn mint_tokens() {
    init_logging();

    let mint_amount = 10u64;

    let program_pubkey =
        try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Fungible-Token-Standard").unwrap();

    let mint_account_pubkey = try_create_mint_account(false).unwrap();

    let previous_mint_details = get_mint_info(&mint_account_pubkey).unwrap();

    let (account_owner_key_pair, account_owner_pubkey, _account_owner_address) =
        generate_new_keypair();

    let balance_account_pubkey = create_balance_account(
        &account_owner_pubkey,
        account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    let resulting_balance = get_balance_account(&balance_account_pubkey).unwrap();

    let mint_instruction = mint_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &balance_account_pubkey,
        &account_owner_pubkey,
        mint_amount,
    )
    .unwrap();

    let mint_transaction = build_transaction(vec![account_owner_key_pair], vec![mint_instruction]);

    let block_transactions = build_and_send_block(vec![mint_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    let resulting_balance = get_balance_account(&balance_account_pubkey).unwrap();

    assert_eq!(resulting_balance.current_balance, mint_amount);

    let mint_details = get_mint_info(&mint_account_pubkey).unwrap();

    assert_eq!(mint_details.supply, MINT_TEST_SUPPLY);

    assert_eq!(mint_details.decimals, previous_mint_details.decimals);
}

#[ignore]
#[serial]
#[test]
fn mint_last_tokens() {
    init_logging();

    let program_pubkey =
        try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Fungible-Token-Standard").unwrap();

    let mint_account_pubkey = try_create_mint_account(true).unwrap();

    let previous_mint_details = get_mint_info(&mint_account_pubkey).unwrap();

    let (account_owner_key_pair, account_owner_pubkey, _account_owner_address) =
        generate_new_keypair();

    let balance_account_pubkey = create_balance_account(
        &account_owner_pubkey,
        account_owner_key_pair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    let resulting_balance = get_balance_account(&balance_account_pubkey).unwrap();

    let mint_instruction = mint_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &balance_account_pubkey,
        &account_owner_pubkey,
        previous_mint_details.supply - previous_mint_details.circulating_supply,
    )
    .unwrap();

    let mint_transaction = build_transaction(vec![account_owner_key_pair], vec![mint_instruction]);

    let block_transactions = build_and_send_block(vec![mint_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    let resulting_balance = get_balance_account(&balance_account_pubkey).unwrap();

    let mint_details = get_mint_info(&mint_account_pubkey).unwrap();

    assert_eq!(
        resulting_balance.current_balance,
        mint_details.circulating_supply,
    );

    assert_eq!(mint_details.circulating_supply, mint_details.supply);

    assert_eq!(mint_details.supply, MINT_TEST_SUPPLY);

    assert!(matches!(mint_details.status, MintStatus::Finished));

    let mint_depleted_instruction = mint_request_instruction(
        &mint_account_pubkey,
        &program_pubkey,
        &balance_account_pubkey,
        &account_owner_pubkey,
        1,
    )
    .unwrap();

    let mint_depleted_transaction = build_transaction(
        vec![account_owner_key_pair],
        vec![mint_depleted_instruction],
    );

    let block_transactions = build_and_send_block(vec![mint_depleted_transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed(_)
    ));
}
