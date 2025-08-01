use std::str::FromStr;

use crate::{
    counter_helpers::{generate_anchoring, get_account_counter},
    counter_instructions::{get_counter_increase_instruction, start_new_counter, CounterData},
    rollback_tests::mine_block,
    ELF_PATH,
};
use arch_program::sanitized::ArchMessage;

use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Status,
};
use arch_test_sdk::{
    constants::{
        BITCOIN_NETWORK, BITCOIN_NODE_ENDPOINT, BITCOIN_NODE_PASSWORD, BITCOIN_NODE_USERNAME,
        NODE1_ADDRESS, PROGRAM_AUTHORITY_FILE_PATH, PROGRAM_FILE_PATH,
    },
    helper::{
        create_and_fund_account_with_faucet, deploy_program, read_account_info,
        send_transactions_and_wait,
    },
    logging::{init_logging, log_scenario_end, log_scenario_start},
};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serial_test::serial;

#[ignore]
#[serial]
#[test]
fn counter_initialization_test() {
    init_logging();

    log_scenario_start(1,
        "Program Deployment & Counter Initialization",
        "Happy Path Scenario : deploying the counter program, then initializing the counter to (1,1) "
    );

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, _) = with_secret_key_file(PROGRAM_AUTHORITY_FILE_PATH)
        .expect("getting caller info should not fail");
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let program_pubkey = deploy_program(
        "E2E-Counter".to_string(),
        ELF_PATH.to_string(),
        program_keypair,
        authority_keypair,
    );

    start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    log_scenario_end(1, "");
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_test() {
    init_logging();

    log_scenario_start(2,
        "Counter Initialization and Increase ( Two overlapping states, in two separate blocks )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it in a separate block "
    );

    let client = ArchRpcClient::new(NODE1_ADDRESS);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let account_info = read_account_info(authority_pubkey);

    println!(
        "authority lamports after funding {:?}",
        account_info.lamports
    );

    let program_pubkey = deploy_program(
        "E2E-Counter".to_string(),
        ELF_PATH.to_string(),
        program_keypair,
        authority_keypair,
    );

    let account_info = read_account_info(authority_pubkey);

    println!(
        "authority lamports after deploying {:?}",
        account_info.lamports
    );

    println!("program_pubkey {:?}", program_pubkey);

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let account_info = read_account_info(authority_pubkey);

    println!(
        "authority lamports after initializing counter {:?}",
        account_info.lamports
    );

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let _block_transactions = send_transactions_and_wait(vec![transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(2, 1));

    log_scenario_end(2, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_transaction_test() {
    init_logging();

    log_scenario_start(3,
        "Counter Initialization and Increase ( Two overlapping states, in the same transaction )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it twice in the same transaction, using two separate instructions"
    );

    let client = ArchRpcClient::new(NODE1_ADDRESS);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let program_pubkey = deploy_program(
        "E2E-Counter".to_string(),
        ELF_PATH.to_string(),
        program_keypair,
        authority_keypair,
    );

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction, second_increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let _block_transactions = send_transactions_and_wait(vec![transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(3, 1));

    log_scenario_end(3, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_block_test() {
    init_logging();

    log_scenario_start(4,
        "Counter Initialization and Increase ( Two overlapping states, in the same block )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it twice in the same block, using two separate transactions"
    );

    let client = ArchRpcClient::new(NODE1_ADDRESS);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let program_pubkey = deploy_program(
        "E2E-Counter".to_string(),
        ELF_PATH.to_string(),
        program_keypair,
        authority_keypair,
    );

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let first_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    println!(
        "TXIDS : first tx {}, second {}",
        first_transaction.txid(),
        second_transaction.txid()
    );
    let _block_transactions =
        send_transactions_and_wait(vec![first_transaction, second_transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(3, 1));

    log_scenario_end(4, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_anchored() {
    init_logging();

    log_scenario_start(15,
        "Counter Initialization and Increase ( 1 Anchored Instruction )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring"
    );

    let client = ArchRpcClient::new(NODE1_ADDRESS);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let program_pubkey = deploy_program(
        "E2E-Counter".to_string(),
        ELF_PATH.to_string(),
        program_keypair,
        authority_keypair,
    );

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let anchoring = generate_anchoring(&account_pubkey);

    mine_block();

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let processed_transactions = send_transactions_and_wait(vec![transaction]);

    println!(
        "Processed transaction id : {}",
        processed_transactions[0].runtime_transaction.txid()
    );

    println!(
        "Transaction status : {:?}",
        processed_transactions[0].status
    );

    assert!(!matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions[0].bitcoin_txid.is_some());

    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string(),
    );
    let rpc =
        Client::new(BITCOIN_NODE_ENDPOINT, userpass).expect("rpc shouldn not fail to be initiated");
    println!(
        "\x1b[1m\x1B[34m First Bitcoin transaction submitted :  : {} \x1b[0m",
        arch_test_sdk::constants::get_explorer_tx_url(
            BITCOIN_NETWORK,
            &processed_transactions[0].bitcoin_txid.unwrap().to_string()
        )
    );

    let _tx_info = rpc
        .get_raw_transaction_info(
            &bitcoin::Txid::from_str(&processed_transactions[0].bitcoin_txid.unwrap().to_string())
                .unwrap(),
            None,
        )
        .unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(2, 1));

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![second_account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let second_processed_transactions = send_transactions_and_wait(vec![second_transaction]);

    println!(
        "Processed transaction id : {}",
        second_processed_transactions[0].runtime_transaction.txid()
    );

    println!(
        "Transaction status : {:?}",
        second_processed_transactions[0].status
    );

    assert!(!matches!(
        second_processed_transactions[0].status,
        Status::Failed { .. }
    ));

    println!(
        "\x1b[1m\x1B[34m First Bitcoin transaction submitted :  : {} \x1b[0m",
        arch_test_sdk::constants::get_explorer_tx_url(
            BITCOIN_NETWORK,
            &second_processed_transactions[0]
                .bitcoin_txid
                .unwrap()
                .to_string()
        )
    );

    let _tx_info = rpc
        .get_raw_transaction_info(
            &bitcoin::Txid::from_str(
                &second_processed_transactions[0]
                    .bitcoin_txid
                    .unwrap()
                    .to_string(),
            )
            .unwrap(),
            None,
        )
        .unwrap();

    log_scenario_end(15, &format!("{:?}", final_account_data));
}
