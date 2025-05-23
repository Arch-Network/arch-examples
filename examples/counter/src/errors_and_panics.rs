use crate::{
    counter_helpers::{generate_anchoring, get_account_counter},
    counter_instructions::{get_counter_increase_instruction, start_new_counter, CounterData},
    ELF_PATH,
};
use arch_program::sanitized::ArchMessage;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
    RollbackStatus, Status,
};
use arch_test_sdk::{
    constants::{BITCOIN_NETWORK, NODE1_ADDRESS, PROGRAM_FILE_PATH},
    helper::{
        create_and_fund_account_with_faucet, deploy_program, read_account_info,
        send_transactions_and_wait,
    },
    logging::{init_logging, log_scenario_end, log_scenario_start},
};
use serial_test::serial;

#[ignore]
#[serial]
#[test]
fn counter_inc_single_instruction_fail() {
    init_logging();

    log_scenario_start(5,
        "Counter Initialization and Increase Failure ( One Instruction to Increase should fail )",
        "Initializing the counter to (1,1), then increasing it in a single instruction, the state shouldn't be updated"
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

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        true,
        false,
        None,
        None,
    );

    let increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![increase_transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(5, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_single_instruction_panic() {
    init_logging();

    log_scenario_start(6,
        "Counter Initialization and Increase Failure ( One Instruction to Increase should panic )",
        "Initializing the counter to (1,1), then increasing it in a single instruction, the state shouldn't be updated"
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

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        true,
        None,
        None,
    );

    let increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![increase_transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(6, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_1st_fail() {
    init_logging();

    log_scenario_start(7,
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, first instruction should fail )",
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction failing. The state shouldn't be updated"
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
        true,
        false,
        None,
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction, second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![increase_transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(7, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_2nd_fail() {
    init_logging();

    log_scenario_start(8,
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, second instruction should fail )",
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction failing. The state shouldn't be updated"
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

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        true,
        false,
        None,
        None,
    );

    let increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction, second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![increase_transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(8, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_1st_panic() {
    init_logging();

    log_scenario_start(9,
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, first instruction should panic )",
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction panicking. The state shouldn't be updated"
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
        true,
        None,
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction, second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![increase_transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(9, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_2nd_panic() {
    init_logging();

    log_scenario_start(10,
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, second instruction should panic )",
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction panicking. The state shouldn't be updated"
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

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        true,
        None,
        None,
    );

    let increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction, second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![increase_transaction]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(10, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_1st_fail() {
    init_logging();

    log_scenario_start(13,
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, first transaction should fail )",
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the first transaction failing. The state should be updated by 2nd transaction"
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
        true,
        false,
        None,
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let first_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![
        first_increase_transaction,
        second_increase_transaction,
    ]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(matches!(
        processed_transactions[1].status,
        Status::Processed
    ));

    assert_eq!(final_account_data, CounterData::new(2, 1));

    log_scenario_end(11, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_2nd_fail() {
    init_logging();

    log_scenario_start(13,
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, second transaction should fail )",
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the second transaction failing. The state should be updated by 1st transaction"
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

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        true,
        false,
        None,
        None,
    );

    let first_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![
        first_increase_transaction,
        second_increase_transaction,
    ]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[1].status,
        Status::Failed { .. }
    ));

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    assert_eq!(final_account_data, CounterData::new(2, 1));

    log_scenario_end(12, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_1st_panic() {
    init_logging();

    log_scenario_start(13,
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, first transaction should panic )",
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the first transaction panicking. The state should be updated by 2nd transaction"
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
        true,
        None,
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let first_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![
        first_increase_transaction,
        second_increase_transaction,
    ]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(matches!(
        processed_transactions[1].status,
        Status::Processed
    ));

    assert_eq!(final_account_data, CounterData::new(2, 1));

    log_scenario_end(13, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_2nd_panic() {
    init_logging();

    log_scenario_start(14,
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, second transaction should panic )",
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the first transaction panicking. The state should be updated by 1st transaction"
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

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        true,
        None,
        None,
    );

    let first_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![
        first_increase_transaction,
        second_increase_transaction,
    ]);

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions[1].status,
        Status::Failed { .. }
    ));

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    assert_eq!(final_account_data, CounterData::new(2, 1));

    log_scenario_end(14, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_anchored_fail() {
    init_logging();

    log_scenario_start(16,
        "Counter Initialization and Increase ( 1 Anchored Instruction )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, and the state shouldn't change"
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

    let anchoring = generate_anchoring(&account_pubkey);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, true)),
        None,
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![transaction]);

    //assert!(processed_transactions[0].bitcoin_txid.is_none());

    // assert!(matches!(
    //     processed_transactions[0].status,
    //     Status::Failed(_)
    // ));

    assert!(matches!(
        processed_transactions[0].rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(16, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_anchored_fail_inc_state() {
    init_logging();

    log_scenario_start(17,
        "Counter Initialization and Increase (  1 Anchored Instruction, 1 State only )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, the second instruction should be rolled back, and the state shouldn't change"
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

    let anchoring = generate_anchoring(&account_pubkey);

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, true)),
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
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
            &[first_increase_istruction, second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![transaction]);

    //assert!(processed_transactions[0].bitcoin_txid.is_none());

    // assert!(matches!(
    //     processed_transactions[0].status,
    //     Status::Failed(_)
    // ));

    assert!(matches!(
        processed_transactions[0].rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(17, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_anchored_fail() {
    init_logging();

    log_scenario_start(18,
        "Counter Initialization and Increase ( 1 Anchored Instruction, 1 State only Instruction )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a failing Bitcoin Transaction Anchoring, and a succeeding state only instruction, the entire Runtime transaction and the state shouldn't change"
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

    let anchoring = generate_anchoring(&account_pubkey);

    let _anchoring_2 = generate_anchoring(&account_pubkey);

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, true)),
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
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
            &[first_increase_istruction, second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![transaction]);

    //assert!(processed_transactions[0].bitcoin_txid.is_none());

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    assert!(matches!(
        processed_transactions[0].rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));

    log_scenario_end(18, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_second_anchored_fail() {
    init_logging();

    log_scenario_start(20,
        "Counter Initialization and Increase (  1 State only Instruction succeeding,1 Anchored Instruction failing)",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a succeeding state only instruction, and a failing anchored instruction, the entire Runtime transaction and the state shouldn't change"
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

    let account_info = read_account_info(account_pubkey);

    let utxo_before_block = account_info.utxo.clone();

    let anchoring = generate_anchoring(&account_pubkey);

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, true)),
        None,
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction, second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions = send_transactions_and_wait(vec![transaction]);

    //assert!(processed_transactions[0].bitcoin_txid.is_none());

    // assert!(matches!(
    //     processed_transactions[0].status,
    //     Status::Failed(_)
    // ));

    assert!(matches!(
        processed_transactions[0].rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));

    let account_info = read_account_info(account_pubkey);

    let utxo_after_block = account_info.utxo.clone();

    assert_eq!(utxo_after_block, utxo_before_block);

    log_scenario_end(20, &format!("{:?}", final_account_data));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_tx_anchored_fail_2nd_succeed() {
    init_logging();

    log_scenario_start(21,
        "Counter Initialization and Increase ( 1 Anchored transaction signaled to fail, 1 Anchored Transaction signaled to succeed (TWO DIFFERENT STATE ACCOUNTS) )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, and the state shouldn't change. The second transaction will try to change another state with an anchoring it should succeed"
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

    let (first_account_pubkey, first_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let first_account_info = read_account_info(first_account_pubkey);

    let second_account_info = read_account_info(second_account_pubkey);

    let first_utxo_before_block = first_account_info.utxo.clone();

    let second_utxo_before_block = second_account_info.utxo.clone();

    let first_anchoring = generate_anchoring(&first_account_pubkey);

    let second_anchoring = generate_anchoring(&second_account_pubkey);

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((first_anchoring.0, first_anchoring.1, true)),
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((second_anchoring.0, second_anchoring.1, false)),
        None,
    );

    let first_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![first_account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![second_account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions =
        send_transactions_and_wait(vec![first_transaction, second_transaction]);

    let first_account_info = read_account_info(first_account_pubkey);

    let second_account_info = read_account_info(second_account_pubkey);

    let first_utxo_after_block = first_account_info.utxo.clone();

    let second_utxo_after_block = second_account_info.utxo.clone();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    assert!(matches!(
        processed_transactions[0].rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    assert!(processed_transactions[1].bitcoin_txid.is_some());

    assert!(matches!(
        processed_transactions[1].status,
        Status::Processed
    ));

    assert!(matches!(
        processed_transactions[1].rollback_status,
        RollbackStatus::NotRolledback
    ));

    //rpc.get_raw_transaction(&tx.txid(), None,None);

    assert_eq!(first_utxo_after_block, first_utxo_before_block);

    assert_ne!(second_utxo_after_block, second_utxo_before_block);
    println!();

    println!("\x1b[1m\x1B[34m Both Bitcoin transactions failed !");

    let final_first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    println!("First Account data {:?}", final_first_account_data);

    assert_eq!(final_first_account_data, CounterData::new(1, 1));

    let final_second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    println!("First Account data {:?}", final_second_account_data);

    assert_eq!(final_second_account_data, CounterData::new(2, 1));

    log_scenario_end(
        21,
        &format!(
            "{:?} === {:?}",
            final_first_account_data, final_second_account_data
        ),
    );
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_tx_anchored_fail_2nd_state_only_succeed() {
    init_logging();

    log_scenario_start(22,
        "Counter Initialization and Increase ( 1 Anchored transaction signaled to fail, 1 state only Transaction signaled to succeed (TWO DIFFERENT STATE ACCOUNTS) )",
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, and the state shouldn't change. The second transaction will try to change another state without an anchoring it should succeed"
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

    let (first_account_pubkey, first_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let first_account_info = read_account_info(first_account_pubkey);

    let second_account_info = read_account_info(second_account_pubkey);

    let first_utxo_before_block = first_account_info.utxo.clone();

    let second_utxo_before_block = second_account_info.utxo.clone();

    let first_anchoring = generate_anchoring(&first_account_pubkey);

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((first_anchoring.0, first_anchoring.1, true)),
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
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
            client.get_best_block_hash().unwrap(),
        ),
        vec![first_account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![second_account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let processed_transactions =
        send_transactions_and_wait(vec![first_transaction, second_transaction]);

    let first_account_info = read_account_info(first_account_pubkey);

    let second_account_info = read_account_info(second_account_pubkey);

    let first_utxo_after_block = first_account_info.utxo.clone();

    let second_utxo_after_block = second_account_info.utxo.clone();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    assert!(matches!(
        processed_transactions[0].rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    assert!(processed_transactions[1].bitcoin_txid.is_none());

    assert!(matches!(
        processed_transactions[1].status,
        Status::Processed
    ));

    assert!(matches!(
        processed_transactions[1].rollback_status,
        RollbackStatus::NotRolledback
    ));

    assert_eq!(first_utxo_after_block, first_utxo_before_block);

    assert_eq!(second_utxo_after_block, second_utxo_before_block);
    println!();

    println!("\x1b[1m\x1B[34m Both Bitcoin transactions failed !");

    let final_first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    println!("First Account data {:?}", final_first_account_data);

    assert_eq!(final_first_account_data, CounterData::new(1, 1));

    let final_second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    println!("First Account data {:?}", final_second_account_data);

    assert_eq!(final_second_account_data, CounterData::new(2, 1));

    log_scenario_end(
        22,
        &format!(
            "{:?} === {:?}",
            final_first_account_data, final_second_account_data
        ),
    );
}
