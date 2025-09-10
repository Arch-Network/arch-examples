use std::str::FromStr;

use crate::{
    counter_helpers::{generate_anchoring, get_account_counter},
    counter_instructions::{get_counter_increase_instruction, start_new_counter, CounterData},
    rollback_tests::mine_block,
    AUTHORITY_FILE_PATH, ELF_PATH, PROGRAM_FILE_PATH,
};
use arch_program::sanitized::ArchMessage;

use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Config,
    ProgramDeployer, Status,
};

use bitcoincore_rpc::{Auth, Client, RpcApi};
use serial_test::serial;

#[ignore]
#[serial]
#[test]
fn counter_initialization_test() {
    println!("Program Deployment & Counter Initialization",);
    println!("Happy Path Scenario : deploying the counter program, then initializing the counter to (1,1) "
    );

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, _) =
        with_secret_key_file(AUTHORITY_FILE_PATH).expect("getting caller info should not fail");

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);
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

    start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_test() {
    println!(
        "Counter Initialization and Increase ( Two overlapping states, in two separate blocks )",
    );
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it in a separate block "
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let account_info = client.read_account_info(authority_pubkey).unwrap();

    println!(
        "authority lamports after funding {:?}",
        account_info.lamports
    );
    let deployer = ProgramDeployer::new(&config);
    let program_pubkey = deployer
        .try_deploy_program(
            "E2E-Counter".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

    let account_info = client.read_account_info(authority_pubkey).unwrap();

    println!(
        "authority lamports after deploying {:?}",
        account_info.lamports
    );

    println!("program_pubkey {:?}", program_pubkey);

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let account_info = client.read_account_info(authority_pubkey).unwrap();

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
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(transaction).unwrap();
    let _block_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(2, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_transaction_test() {
    println!(
        "Counter Initialization and Increase ( Two overlapping states, in the same transaction )",
    );
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it twice in the same transaction, using two separate instructions"
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
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(transaction).unwrap();
    let _block_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(3, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_block_test() {
    println!("Counter Initialization and Increase ( Two overlapping states, in the same block )",);
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it twice in the same block, using two separate transactions"
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

    let first_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[first_increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
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
        config.network,
    )
    .expect("Failed to build and sign transaction");

    println!(
        "TXIDS : first tx {}, second {}",
        first_transaction.txid(),
        second_transaction.txid()
    );

    let txids = client
        .send_transactions(vec![first_transaction, second_transaction])
        .unwrap();
    let _block_transactions = client.wait_for_processed_transactions(txids).unwrap();

    let final_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(final_account_data, CounterData::new(3, 1));
}

#[ignore]
#[serial]
#[test]
fn counter_init_and_inc_anchored() {
    println!("Counter Initialization and Increase ( 1 Anchored Instruction )",);
    println!(
        "Happy Path Scenario : Initializing the counter to (1,1), then increasing it with a Bitcoin Transaction Anchoring"
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
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(transaction).unwrap();
    let processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Processed transaction id : {}",
        processed_transaction.runtime_transaction.txid()
    );

    println!("Transaction status : {:?}", processed_transaction.status);

    assert!(!matches!(
        processed_transaction.status,
        Status::Failed { .. }
    ));

    assert!(processed_transaction.bitcoin_txid.is_some());

    let userpass = Auth::UserPass(config.node_username, config.node_password);
    let rpc =
        Client::new(&config.node_endpoint, userpass).expect("rpc shouldn not fail to be initiated");

    let _tx_info = rpc
        .get_raw_transaction_info(
            &bitcoin::Txid::from_str(&processed_transaction.bitcoin_txid.unwrap().to_string())
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
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(second_transaction).unwrap();
    let second_processed_transaction = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Processed transaction id : {}",
        second_processed_transaction.runtime_transaction.txid()
    );

    println!(
        "Transaction status : {:?}",
        second_processed_transaction.status
    );

    assert!(!matches!(
        second_processed_transaction.status,
        Status::Failed { .. }
    ));

    let _tx_info = rpc
        .get_raw_transaction_info(
            &bitcoin::Txid::from_str(
                &second_processed_transaction
                    .bitcoin_txid
                    .unwrap()
                    .to_string(),
            )
            .unwrap(),
            None,
        )
        .unwrap();
}
