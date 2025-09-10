// /* -------------------------------------------------------------------------- */
// /*                ROLLBACK TESTS IN CASE OF RBF (REGTEST ONLY)                */
// /* -------------------------------------------------------------------------- */
use std::{str::FromStr, thread, time::Duration};

use arch_program::sanitized::ArchMessage;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Config,
    ProgramDeployer, Status,
};

use bitcoin::{address::NetworkChecked, Address, BlockHash, Network, Txid};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serial_test::serial;

use crate::{
    counter_helpers::{generate_anchoring, get_account_counter},
    counter_instructions::{get_counter_increase_instruction, start_new_counter, CounterData},
    ELF_PATH, MINING_ADDRESS, PROGRAM_FILE_PATH,
};

pub const WAIT_FOR_ROLLBACK: u8 = 10;

pub(crate) fn mine_block() -> BlockHash {
    let config = Config::localnet();

    let userpass = Auth::UserPass(config.node_username, config.node_password);

    let rpc =
        Client::new(&config.node_endpoint, userpass).expect("rpc shouldn not fail to be initiated");

    let mining_address: Address<NetworkChecked> = MINING_ADDRESS
        .parse::<Address<_>>()
        .unwrap()
        .require_network(Network::Regtest)
        .unwrap();

    let mined_block = rpc.generate_to_address(1, &mining_address).unwrap();

    mined_block[0]
}

fn connect_nodes() {
    let config = Config::localnet();

    let userpass = Auth::UserPass(config.node_username, config.node_password);

    let rpc_node1: Client =
        Client::new("http://127.0.0.1:18443/wallet/testwallet", userpass.clone())
            .expect("rpc shouldn not fail to be initiated");

    let rpc_node2: Client =
        Client::new("http://127.0.0.1:18453/wallet/testwallet", userpass.clone())
            .expect("rpc shouldn not fail to be initiated");

    let connection_count_node1 = rpc_node1.get_connection_count().unwrap();
    let connection_count_node2 = rpc_node2.get_connection_count().unwrap();

    if connection_count_node1 == 1 && connection_count_node2 == 1 {
        println!("Nodes already connected");
        return;
    }

    match rpc_node2.add_node("127.0.0.1:18444") {
        Ok(_) => {
            println!("Node added to node2");
        }
        Err(e) => println!("Error removing node from node2: {:?}", e),
    }
    match rpc_node2.onetry_node("127.0.0.1:18444") {
        Ok(_) => {
            println!("Node added to node2");
        }
        Err(e) => println!("Error removing node from node2: {:?}", e),
    }

    thread::sleep(std::time::Duration::from_secs(10));

    let connection_count_node1 = rpc_node1.get_connection_count().unwrap();
    let connection_count_node2 = rpc_node2.get_connection_count().unwrap();

    assert_eq!(connection_count_node1, 1);
    assert_eq!(connection_count_node2, 1);
}

fn isolate_nodes() {
    let config = Config::localnet();

    let userpass = Auth::UserPass(config.node_username, config.node_password);

    let rpc_node1: Client =
        Client::new("http://127.0.0.1:18443/wallet/testwallet", userpass.clone())
            .expect("rpc shouldn not fail to be initiated");

    let rpc_node2: Client =
        Client::new("http://127.0.0.1:18453/wallet/testwallet", userpass.clone())
            .expect("rpc shouldn not fail to be initiated");

    let connection_count_node1 = rpc_node1.get_connection_count().unwrap();
    let connection_count_node2 = rpc_node2.get_connection_count().unwrap();

    if connection_count_node1 == 0 && connection_count_node2 == 0 {
        return;
    }

    match rpc_node2.remove_node("127.0.0.1:18444") {
        Ok(_) => {
            println!("Node removed from node2");
        }
        Err(e) => println!("Error removing node from node2: {:?}", e),
    }
    match rpc_node2.disconnect_node("127.0.0.1:18444") {
        Ok(_) => {
            println!("Node disconnected from node2");
        }
        Err(e) => println!("Error disconnecting node from node2: {:?}", e),
    }

    thread::sleep(std::time::Duration::from_secs(10));

    let connection_count_node1 = rpc_node1.get_connection_count().unwrap();
    let connection_count_node2 = rpc_node2.get_connection_count().unwrap();

    assert_eq!(connection_count_node1, 0);
    assert_eq!(connection_count_node2, 0);
}

#[ignore]
#[serial]
#[test]
fn single_utxo_rbf_two_accounts() {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    println!("2 Counters, same utxo replaced by a greater fee",);
    println!("Roll Back scenario : Same utxo is used to update different accounts, the replaced transaction should be rolled back"
    );

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

    println!("First Counter Initialization and increase");

    let (first_account_pubkey, first_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &first_authority_keypair).unwrap();

    println!("Second Counter Initialization and increase");

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &second_authority_keypair).unwrap();

    let anchoring = generate_anchoring(&first_account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    println!(
        "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
        btc_block_hash.as_raw_hash(),
    );
    println!();

    println!("Increasing the first counter using the unique utxo");

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![first_account_keypair, first_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(transaction).unwrap();
    let processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "First increase processed transaction id : {}\nStatus: {:?}",
        processed_transactions.txid(),
        processed_transactions.status
    );

    assert!(!matches!(
        processed_transactions.status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions.bitcoin_txid.is_some());

    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));

    println!("Increasing the second counter using the same unique utxo",);

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &second_authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(second_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![second_account_keypair, second_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(second_transaction).unwrap();
    let second_processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions.txid()
    );

    assert!(!matches!(
        second_processed_transactions.status,
        Status::Failed { .. }
    ));

    let _ = mine_block();

    thread::sleep(std::time::Duration::from_secs(10));

    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(1, 1));
    assert_eq!(second_account_data, CounterData::new(2, 1));
}

#[ignore]
#[serial]
#[test]
fn single_utxo_rbf_three_accounts() {
    println!("3 Counters, same utxo replaced twice by a greater fee",);
    println!(
        "Roll Back scenario : Same utxo is used to update different accounts, the replaced transactions should be rolled back"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(".program.jso").expect("getting caller info should not fail");

    let (first_authority_keypair, first_authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&first_authority_keypair)
        .unwrap();

    let (second_authority_keypair, second_authority_pubkey, _) =
        generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&second_authority_keypair)
        .unwrap();

    let (third_authority_keypair, third_authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&third_authority_keypair)
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

    println!("First Counter Initialization and increase");

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &first_authority_keypair).unwrap();

    println!("Second Counter Initialization and increase");

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &second_authority_keypair).unwrap();

    println!("Third Counter Initialization and increase");

    let (third_account_pubkey, third_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &third_authority_keypair).unwrap();

    let anchoring = generate_anchoring(&account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    println!(
        "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
        btc_block_hash.as_raw_hash()
    );
    println!();

    println!("Increasing the first counter using the unique utxo");

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(5000),
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, first_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(transaction).unwrap();
    let processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "First increase processed transaction id : {}",
        processed_transactions.txid()
    );

    assert!(!matches!(
        processed_transactions.status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions.bitcoin_txid.is_some());

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));

    println!("Increasing the second counter using the same unique utxo");

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &second_authority_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(second_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![second_account_keypair, second_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(second_transaction).unwrap();
    let second_processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions.txid()
    );

    assert!(!matches!(
        second_processed_transactions.status,
        Status::Failed { .. }
    ));

    println!("Increasing the third counter using the same unique utxo");

    let third_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &third_account_pubkey,
        &third_authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let third_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[third_increase_istruction],
            Some(third_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![third_account_keypair, third_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(third_transaction).unwrap();
    let third_processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Third increase processed transaction id : {} {:?}",
        third_processed_transactions.txid(),
        third_processed_transactions.status
    );

    assert!(!matches!(
        third_processed_transactions.status,
        Status::Failed { .. }
    ));

    let _btc_block_hash = mine_block();

    thread::sleep(std::time::Duration::from_secs(10));

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    let third_account_data = get_account_counter(&third_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(1, 1));
    assert_eq!(second_account_data, CounterData::new(1, 1));
    assert_eq!(third_account_data, CounterData::new(2, 1));

    //let second_account_data = get_account_counter(&second_account_pubkey).unwrap();
}

#[ignore]
#[serial]
#[test]
fn rbf_orphan_arch_txs() {
    println!("2 Counters, same utxo replaced by a greater fee, w/ orphan arch tx",);
    println!(
        "Roll Back scenario : First account updated with utxo, then updated again without anchoring. Sane utxo is then used to update another account in RBF"
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

    println!("First Counter Initialization and increase");

    let (first_account_pubkey, first_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &first_authority_keypair).unwrap();

    println!("Second Counter Initialization and increase");

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &second_authority_keypair).unwrap();

    let anchoring = generate_anchoring(&first_account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    println!(
        "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
        btc_block_hash.as_raw_hash()
    );
    println!();

    println!("Increasing the first counter using the unique utxo");

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![first_account_keypair, first_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(transaction).unwrap();
    let processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "First increase processed transaction id : {}",
        processed_transactions.txid()
    );

    assert!(!matches!(
        processed_transactions.status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions.bitcoin_txid.is_some());

    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));

    println!("Increasing the first counter again without anchoring");

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![first_account_keypair, first_authority_keypair],
        config.network,
    )
    .unwrap();
    let txid = client.send_transaction(transaction).unwrap();

    let processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Second increase for first account processed transaction id : {}",
        processed_transactions.txid()
    );

    assert!(!matches!(
        processed_transactions.status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions.bitcoin_txid.is_none());

    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(3, 1));

    println!("Increasing the second counter using the same unique utxo",);

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &second_authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(second_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![second_account_keypair, second_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(second_transaction).unwrap();
    let second_processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions.txid()
    );

    assert!(!matches!(
        second_processed_transactions.status,
        Status::Failed { .. }
    ));

    let _ = mine_block();

    thread::sleep(std::time::Duration::from_secs(10));

    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(1, 1));

    assert_eq!(second_account_data, CounterData::new(2, 1));

    //let second_account_data = get_account_counter(&second_account_pubkey).unwrap();
}

#[ignore]
#[serial]
#[test]
fn rbf_reorg() {
    println!("2 Counters, same utxo replaced by a greater fee, w/ orphan arch tx",);
    println!(
        "Roll Back scenario : First account updated with utxo, then updated again without anchoring. Same utxo is then used to update another account in RBF"
    );

    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    connect_nodes();

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

    println!("First Counter Initialization and increase");

    let (first_account_pubkey, first_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &first_authority_keypair).unwrap();

    println!("Second Counter Initialization and increase");

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &second_authority_keypair).unwrap();

    let anchoring = generate_anchoring(&first_account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    println!(
        "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
        btc_block_hash.as_raw_hash()
    );
    println!();

    println!("Increasing the first counter using the unique utxo");

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![first_account_keypair, first_authority_keypair],
        config.network,
    )
    .unwrap();

    let txid = client.send_transaction(transaction).unwrap();
    let processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "First increase processed transaction id : {}",
        processed_transactions.txid()
    );

    // println!("First transaction : {:?}", processed_transactions);

    assert!(!matches!(
        processed_transactions.status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions.bitcoin_txid.is_some());

    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();
    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));
    assert_eq!(second_account_data, CounterData::new(1, 1));
    println!("First account data : {:?}", first_account_data);
    println!("Second account data : {:?}", second_account_data);

    let userpass = Auth::UserPass(config.node_username.clone(), config.node_password.clone());
    let rpc_node1 = Client::new("http://127.0.0.1:18443/wallet/testwallet", userpass.clone())
        .expect("rpc shouldn not fail to be initiated");
    let rpc_node2 = Client::new("http://127.0.0.1:18453/wallet/testwallet", userpass.clone())
        .expect("rpc shouldn not fail to be initiated");

    let first_txid =
        Txid::from_str(&processed_transactions.bitcoin_txid.unwrap().to_string()).unwrap();

    let first_tx = rpc_node1.get_raw_transaction(&first_txid, None).unwrap();
    rpc_node2.send_raw_transaction(&first_tx).unwrap();

    isolate_nodes();

    println!("Increasing the second counter using the same unique utxo",);

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        &second_authority_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(second_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![second_account_keypair, second_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(second_transaction).unwrap();
    let second_processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions.txid()
    );

    assert!(!matches!(
        second_processed_transactions.status,
        Status::Failed { .. }
    ));

    thread::sleep(Duration::from_secs(5));
    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    println!(
        "First account data : {:?}",
        client.read_account_info(first_account_pubkey).unwrap()
    );
    println!(
        "Second account data : {:?}",
        client.read_account_info(second_account_pubkey).unwrap()
    );
    assert_eq!(first_account_data, CounterData::new(1, 1));
    assert_eq!(second_account_data, CounterData::new(2, 1));

    let userpass = Auth::UserPass(config.node_username, config.node_password);
    let rpc_node2 = Client::new("http://127.0.0.1:18453/wallet/testwallet", userpass.clone())
        .expect("rpc shouldn not fail to be initiated");
    rpc_node2
        .generate_to_address(
            3,
            &bitcoin::Address::from_str(MINING_ADDRESS)
                .unwrap()
                .require_network(Network::Regtest)
                .unwrap(),
        )
        .unwrap();
    connect_nodes();

    println!("Increasing the first counter again without anchoring");

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &first_account_pubkey,
        &first_authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[increase_istruction],
            Some(first_authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![first_account_keypair, first_authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");
    let txid = client.send_transaction(transaction).unwrap();
    let processed_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "Second increase for first account processed transaction id : {}",
        processed_transactions.txid()
    );

    assert!(!matches!(
        processed_transactions.status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions.bitcoin_txid.is_none());

    thread::sleep(Duration::from_secs(5));
    let first_account_data = get_account_counter(&first_account_pubkey).unwrap();
    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    println!(
        "First account : {:?}",
        client.read_account_info(first_account_pubkey).unwrap()
    );
    println!(
        "Second account : {:?}",
        client.read_account_info(second_account_pubkey).unwrap()
    );

    assert_eq!(first_account_data, CounterData::new(3, 1));
    assert_eq!(second_account_data, CounterData::new(1, 1));
}
