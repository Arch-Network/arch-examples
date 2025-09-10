use crate::{
    counter_helpers::{generate_anchoring, get_account_counter},
    counter_instructions::{get_counter_increase_instruction, start_new_counter, CounterData},
    ELF_PATH, PROGRAM_FILE_PATH,
};
use arch_program::sanitized::ArchMessage;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Config,
    ProgramDeployer, RollbackStatus, Status,
};
use serial_test::serial;

#[ignore]
#[serial]
#[test]
fn counter_inc_single_instruction_fail() {
    println!(
        "Counter Initialization and Increase Failure ( One Instruction to Increase should fail )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it in a single instruction, the state shouldn't be updated"
    );
    let config = Config::localnet();

    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    // Print bitcoin network
    println!("Bitcoin network: {:?}", config.network);

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(increase_transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transaction.status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_single_instruction_panic() {
    println!(
        "Counter Initialization and Increase Failure ( One Instruction to Increase should panic )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it in a single instruction, the state shouldn't be updated"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(increase_transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transaction.status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_1st_fail() {
    println!(
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, first instruction should fail )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction failing. The state shouldn't be updated"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(increase_transaction).unwrap();
    let processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transactions.status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_2nd_fail() {
    println!(
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, second instruction should fail )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction failing. The state shouldn't be updated"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

    let program_info = client.read_account_info(program_pubkey).unwrap();
    println!("program_info: {:?}", program_info.is_executable);
    println!("program_info: {:?}", program_info.data.len());
    println!("program_info: {:?}", program_info.utxo);
    println!("program_info: {:?}", program_info.owner);

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(increase_transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transaction.status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_1st_panic() {
    println!(
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, first instruction should panic )",
    );

    println!(
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction panicking. The state shouldn't be updated"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(increase_transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transaction.status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_instructions_2nd_panic() {
    println!(
        "Counter Initialization and Increase Failure ( Two Instructions to Increase, second instruction should panic )",

    );
    println!(
        "Initializing the counter to (1,1), then increasing it twice within the same transaction, with the first instruction panicking. The state shouldn't be updated"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(increase_transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert!(matches!(
        processed_transaction.status,
        Status::Failed { .. }
    ));

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_1st_fail() {
    println!(
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, first transaction should fail )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the first transaction failing. The state should be updated by 2nd transaction"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txids = client
        .send_transactions(vec![
            first_increase_transaction,
            second_increase_transaction,
        ])
        .unwrap();
    let processed_transactions = client.wait_for_processed_transactions(txids).unwrap();

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
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_2nd_fail() {
    println!(
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, second transaction should fail )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the second transaction failing. The state should be updated by 1st transaction"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txids = client
        .send_transactions(vec![
            first_increase_transaction,
            second_increase_transaction,
        ])
        .unwrap();
    let processed_transactions = client.wait_for_processed_transactions(txids).unwrap();

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
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_1st_panic() {
    println!(
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, first transaction should panic )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the first transaction panicking. The state should be updated by 2nd transaction"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .unwrap();

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .unwrap();

    let txids = client
        .send_transactions(vec![
            first_increase_transaction,
            second_increase_transaction,
        ])
        .unwrap();
    let processed_transactions = client.wait_for_processed_transactions(txids).unwrap();

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
}

#[ignore]
#[serial]
#[test]
fn counter_inc_two_transactions_2nd_panic() {
    println!(
        "Counter Initialization and Increase Failure ( Two separate transactions to Increase, second transaction should panic )",
    );
    println!(
        "Initializing the counter to (1,1), then increasing it twice in two separate transactions, with the first transaction panicking. The state should be updated by 1st transaction"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();
    let deployer = ProgramDeployer::new(&config);
    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txids = client
        .send_transactions(vec![
            first_increase_transaction,
            second_increase_transaction,
        ])
        .unwrap();
    let processed_transactions = client.wait_for_processed_transactions(txids).unwrap();

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
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_anchored_fail() {
    println!("Counter Initialization and Increase ( 1 Anchored Instruction )",);
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, and the state shouldn't change"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();
    let deployer = ProgramDeployer::new(&config);
    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    // assert!(processed_transactions.bitcoin_txid.is_none());

    // assert!(matches!(
    //     processed_transactions.status,
    //     Status::Failed(_)
    // ));
    println!("processed_transaction {:?}", processed_transaction);

    assert!(matches!(
        processed_transaction.rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_anchored_fail_inc_state() {
    println!("Counter Initialization and Increase (  1 Anchored Instruction, 1 State only )",);
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, the second instruction should be rolled back, and the state shouldn't change"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();
    let deployer = ProgramDeployer::new(&config);
    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .unwrap();

    let txid = client.send_transaction(transaction).unwrap();

    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    //assert!(processed_transactions.bitcoin_txid.is_none());

    // assert!(matches!(
    //     processed_transactions.status,
    //     Status::Failed(_)
    // ));

    assert!(matches!(
        processed_transaction.rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_anchored_fail() {
    println!(
        "Counter Initialization and Increase ( 1 Anchored Instruction, 1 State only Instruction )",
    );
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a failing Bitcoin Transaction Anchoring, and a succeeding state only instruction, the entire Runtime transaction and the state shouldn't change"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();
    let deployer = ProgramDeployer::new(&config);
    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    //assert!(processed_transactions.bitcoin_txid.is_none());
    println!("processed_transaction: {:?}", processed_transaction);

    assert!(matches!(processed_transaction.status, Status::Processed));

    assert!(matches!(
        processed_transaction.rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_second_anchored_fail() {
    println!(
        "Counter Initialization and Increase (  1 State only Instruction succeeding,1 Anchored Instruction failing)",
    );
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a succeeding state only instruction, and a failing anchored instruction, the entire Runtime transaction and the state shouldn't change"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();
    let deployer = ProgramDeployer::new(&config);
    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let account_info = client.read_account_info(account_pubkey).unwrap();

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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();
    println!("processed_transaction: {:?}", processed_transaction);

    //assert!(processed_transactions.bitcoin_txid.is_none());

    // assert!(matches!(
    //     processed_transactions.status,
    //     Status::Failed(_)
    // ));

    assert!(matches!(
        processed_transaction.rollback_status,
        RollbackStatus::Rolledback(_)
    ));

    println!();

    println!("\x1b[1m\x1B[34m Bitcoin transaction failed !");

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(1, 1));

    let account_info = client.read_account_info(account_pubkey).unwrap();

    let utxo_after_block = account_info.utxo.clone();

    assert_eq!(utxo_after_block, utxo_before_block);
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_tx_anchored_fail_2nd_succeed() {
    println!(
        "Counter Initialization and Increase ( 1 Anchored transaction signaled to fail, 1 Anchored Transaction signaled to succeed (TWO DIFFERENT STATE ACCOUNTS) )",

    );
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, and the state shouldn't change. The second transaction will try to change another state with an anchoring it should succeed"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (first_authority_keypair, first_authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&first_authority_keypair)
        .unwrap();

    let (second_authority_keypair, second_authority_pubkey, _) =
        generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&second_authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            first_authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

    let (first_account_pubkey, first_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &first_authority_keypair).unwrap();

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &second_authority_keypair).unwrap();

    let first_account_info = client.read_account_info(first_account_pubkey).unwrap();

    let second_account_info = client.read_account_info(second_account_pubkey).unwrap();

    let first_utxo_before_block = first_account_info.utxo.clone();

    let second_utxo_before_block = second_account_info.utxo.clone();

    let first_anchoring = generate_anchoring(&first_account_pubkey);

    let second_anchoring = generate_anchoring(&second_account_pubkey);

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        Some((first_anchoring.0, first_anchoring.1, true)),
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &second_authority_pubkey,
        false,
        false,
        Some((second_anchoring.0, second_anchoring.1, false)),
        None,
    );

    let first_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![first_account_keypair, first_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(second_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![second_account_keypair, second_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txids = client
        .send_transactions(vec![first_transaction, second_transaction])
        .unwrap();
    let processed_transactions = client.wait_for_processed_transactions(txids).unwrap();

    let first_account_info = client.read_account_info(first_account_pubkey).unwrap();

    let second_account_info = client.read_account_info(second_account_pubkey).unwrap();

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
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_two_inc_tx_anchored_fail_2nd_state_only_succeed() {
    println!(
        "Counter Initialization and Increase ( 1 Anchored transaction signaled to fail, 1 state only Transaction signaled to succeed (TWO DIFFERENT STATE ACCOUNTS) )",

    );
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring, the BTC anchoring should fail, and the state shouldn't change. The second transaction will try to change another state without an anchoring it should succeed"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (first_authority_keypair, first_authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&first_authority_keypair)
        .unwrap();

    let (second_authority_keypair, second_authority_pubkey, _) =
        generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&second_authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);
    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            first_authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

    let (first_account_pubkey, first_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &first_authority_keypair).unwrap();

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &second_authority_keypair).unwrap();

    let first_account_info = client.read_account_info(first_account_pubkey).unwrap();

    let second_account_info = client.read_account_info(second_account_pubkey).unwrap();

    let first_utxo_before_block = first_account_info.utxo.clone();

    let second_utxo_before_block = second_account_info.utxo.clone();

    let first_anchoring = generate_anchoring(&first_account_pubkey);

    let first_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        Some((first_anchoring.0, first_anchoring.1, true)),
        None,
    );

    let second_increase_instruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &second_authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let first_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![first_account_keypair, first_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_instruction],
            Some(second_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![second_account_keypair, second_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txids = client
        .send_transactions(vec![first_transaction, second_transaction])
        .unwrap();
    let processed_transactions = client.wait_for_processed_transactions(txids).unwrap();

    let first_account_info = client.read_account_info(first_account_pubkey).unwrap();

    let second_account_info = client.read_account_info(second_account_pubkey).unwrap();

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

    println!(
        "processed_transactions[1] {} rollback_status {:?}",
        processed_transactions[1].txid(),
        processed_transactions[1].rollback_status
    );

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
}
