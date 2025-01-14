/* -------------------------------------------------------------------------- */
/*                ROLLBACK TESTS IN CASE OF RBF (REGTEST ONLY)                */
/* -------------------------------------------------------------------------- */

use std::{str::FromStr, thread, time::Duration};

use arch_sdk::{
    constants::{
        BITCOIN_NODE1_ADDRESS, BITCOIN_NODE1_P2P_ADDRESS, BITCOIN_NODE2_ADDRESS,
        BITCOIN_NODE_ENDPOINT, BITCOIN_NODE_PASSWORD, BITCOIN_NODE_USERNAME, MINING_ADDRESS,
        NODE1_ADDRESS, PROGRAM_FILE_PATH,
    },
    helper::{
        build_and_send_block, build_transaction, fetch_processed_transactions, init_logging,
        log_scenario_end, log_scenario_start, print_title, read_account_info, try_deploy_program,
    },
    processed_transaction::Status,
};
use bitcoin::{address::NetworkChecked, Address, BlockHash, Network, Txid};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use electrs_client::ElectrsClient;
use serial_test::serial;

use crate::{
    counter_helpers::{generate_anchoring_psbt, get_account_counter},
    counter_instructions::{get_counter_increase_instruction, start_new_counter, CounterData},
    ELF_PATH,
};

pub const WAIT_FOR_ROLLBACK: u8 = 10;

pub(crate) fn mine_block() -> BlockHash {
    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string(),
    );
    let rpc =
        Client::new(BITCOIN_NODE_ENDPOINT, userpass).expect("rpc shouldn not fail to be initiated");

    let mining_address: Address<NetworkChecked> = MINING_ADDRESS
        .parse::<Address<_>>()
        .unwrap()
        .require_network(Network::Regtest)
        .unwrap();

    let mined_block = rpc.generate_to_address(1, &mining_address).unwrap();

    mined_block[0]
}

fn connect_nodes() {
    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string(),
    );

    let rpc_node1: Client = Client::new(BITCOIN_NODE1_ADDRESS, userpass.clone())
        .expect("rpc shouldn not fail to be initiated");

    let rpc_node2: Client = Client::new(BITCOIN_NODE2_ADDRESS, userpass.clone())
        .expect("rpc shouldn not fail to be initiated");

    let connection_count_node1 = rpc_node1.get_connection_count().unwrap();
    let connection_count_node2 = rpc_node2.get_connection_count().unwrap();

    if connection_count_node1 == 1 && connection_count_node2 == 1 {
        println!("Nodes already connected");
        return;
    }

    match rpc_node2.add_node(&BITCOIN_NODE1_P2P_ADDRESS) {
        Ok(_) => {
            println!("Node added to node2");
        }
        Err(e) => println!("Error removing node from node2: {:?}", e),
    }
    match rpc_node2.onetry_node(&BITCOIN_NODE1_P2P_ADDRESS) {
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
    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string(),
    );

    let rpc_node1: Client = Client::new(BITCOIN_NODE1_ADDRESS, userpass.clone())
        .expect("rpc shouldn not fail to be initiated");

    let rpc_node2: Client = Client::new(BITCOIN_NODE2_ADDRESS, userpass.clone())
        .expect("rpc shouldn not fail to be initiated");

    let connection_count_node1 = rpc_node1.get_connection_count().unwrap();
    let connection_count_node2 = rpc_node2.get_connection_count().unwrap();

    if connection_count_node1 == 0 && connection_count_node2 == 0 {
        return;
    }

    match rpc_node2.remove_node(&BITCOIN_NODE1_P2P_ADDRESS) {
        Ok(_) => {
            println!("Node removed from node2");
        }
        Err(e) => println!("Error removing node from node2: {:?}", e),
    }
    match rpc_node2.disconnect_node(&BITCOIN_NODE1_P2P_ADDRESS) {
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
    init_logging();

    log_scenario_start(23,
        "2 Counters, same utxo replaced by a greater fee",
        "Roll Back scenario : Same utxo is used to update different accounts, the replaced transaction should be rolled back"
    );

    let program_pubkey = try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "E2E-Counter").unwrap();

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    print_title("Second Counter Initialization and increase", 5);

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1).unwrap();

    let anchoring = generate_anchoring_psbt(&account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    print_title(
        &format!(
            "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
            btc_block_hash.as_raw_hash()
        ),
        4,
    );
    println!();

    print_title("Increasing the first counter using the unique utxo", 5);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

    let block_transactions = build_and_send_block(vec![transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    println!(
        "First increase processed transaction id : {}",
        processed_transactions[0].txid()
    );

    assert!(!matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions[0].bitcoin_txid.is_some());

    println!("\x1b[1m\x1B[34m First Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",processed_transactions[0].bitcoin_txid.clone().unwrap());

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));

    print_title(
        "Increasing the second counter using the same unique utxo",
        5,
    );

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let second_transaction = build_transaction(
        vec![second_account_keypair],
        vec![second_increase_istruction],
    );

    let second_block_transactions = build_and_send_block(vec![second_transaction]);

    let second_processed_transactions =
        fetch_processed_transactions(second_block_transactions).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions[0].txid()
    );

    assert!(!matches!(
        second_processed_transactions[0].status,
        Status::Failed { .. }
    ));

    let _ = mine_block();

    println!("\x1b[1m\x1B[34m Second Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",second_processed_transactions[0].bitcoin_txid.clone().unwrap());

    thread::sleep(std::time::Duration::from_secs(10));

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(1, 1));
    assert_eq!(second_account_data, CounterData::new(2, 1));

    log_scenario_end(23, &format!("{:?}", first_account_data));
}

#[ignore]
#[serial]
#[test]
fn single_utxo_rbf_three_accounts() {
    init_logging();

    log_scenario_start(24,
        "3 Counters, same utxo replaced twice by a greater fee",
        "Roll Back scenario : Same utxo is used to update different accounts, the replaced transactions should be rolled back"
    );

    let program_pubkey = try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "E2E-Counter").unwrap();

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    print_title("Second Counter Initialization and increase", 5);

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1).unwrap();

    print_title("Third Counter Initialization and increase", 5);

    let (third_account_pubkey, third_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1).unwrap();

    let anchoring = generate_anchoring_psbt(&account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    print_title(
        &format!(
            "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
            btc_block_hash.as_raw_hash()
        ),
        4,
    );
    println!();

    print_title("Increasing the first counter using the unique utxo", 5);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(5000),
    );

    let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

    let block_transactions = build_and_send_block(vec![transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    println!(
        "First increase processed transaction id : {}",
        processed_transactions[0].txid()
    );

    assert!(!matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions[0].bitcoin_txid.is_some());

    println!("\x1b[1m\x1B[34m First Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",processed_transactions[0].bitcoin_txid.clone().unwrap());

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));

    print_title(
        "Increasing the second counter using the same unique utxo",
        5,
    );

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let second_transaction = build_transaction(
        vec![second_account_keypair],
        vec![second_increase_istruction],
    );

    let second_block_transactions = build_and_send_block(vec![second_transaction]);

    let second_processed_transactions =
        fetch_processed_transactions(second_block_transactions).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions[0].txid()
    );

    assert!(!matches!(
        second_processed_transactions[0].status,
        Status::Failed { .. }
    ));

    print_title("Increasing the third counter using the same unique utxo", 5);

    let third_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &third_account_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let third_transaction =
        build_transaction(vec![third_account_keypair], vec![third_increase_istruction]);

    let third_block_transactions = build_and_send_block(vec![third_transaction]);

    let third_processed_transactions =
        fetch_processed_transactions(third_block_transactions).unwrap();

    println!(
        "Third increase processed transaction id : {} {:?}",
        third_processed_transactions[0].txid(),
        third_processed_transactions[0].status
    );

    assert!(!matches!(
        third_processed_transactions[0].status,
        Status::Failed { .. }
    ));

    let _btc_block_hash = mine_block();

    println!("\x1b[1m\x1B[34m Third Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",third_processed_transactions[0].bitcoin_txid.clone().unwrap());

    thread::sleep(std::time::Duration::from_secs(10));

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    let third_account_data = get_account_counter(&third_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(1, 1));
    assert_eq!(second_account_data, CounterData::new(1, 1));
    assert_eq!(third_account_data, CounterData::new(2, 1));

    //let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    log_scenario_end(
        25,
        &format!(
            "{:?} / {:?} / {:?}",
            first_account_data, second_account_data, third_account_data
        ),
    );
}

#[ignore]
#[serial]
#[test]
fn rbf_orphan_arch_txs() {
    init_logging();

    log_scenario_start(25,
        "2 Counters, same utxo replaced by a greater fee, w/ orphan arch tx",
        "Roll Back scenario : First account updated with utxo, then updated again without anchoring. Sane utxo is then used to update another account in RBF"
    );

    let program_pubkey = try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "E2E-Counter").unwrap();

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    print_title("Second Counter Initialization and increase", 5);

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1).unwrap();

    let anchoring = generate_anchoring_psbt(&account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    print_title(
        &format!(
            "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
            btc_block_hash.as_raw_hash()
        ),
        4,
    );
    println!();

    print_title("Increasing the first counter using the unique utxo", 5);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

    let block_transactions = build_and_send_block(vec![transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    println!(
        "First increase processed transaction id : {}",
        processed_transactions[0].txid()
    );

    assert!(!matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions[0].bitcoin_txid.is_some());

    println!("\x1b[1m\x1B[34m First Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",processed_transactions[0].bitcoin_txid.clone().unwrap());

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));

    print_title("Increasing the first counter again without anchoring", 5);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        None,
        None,
    );

    let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

    let block_transactions = build_and_send_block(vec![transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    println!(
        "Second increase for first account processed transaction id : {}",
        processed_transactions[0].txid()
    );

    assert!(!matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions[0].bitcoin_txid.is_none());

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(3, 1));

    print_title(
        "Increasing the second counter using the same unique utxo",
        5,
    );

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let second_transaction = build_transaction(
        vec![second_account_keypair],
        vec![second_increase_istruction],
    );

    let second_block_transactions = build_and_send_block(vec![second_transaction]);

    let second_processed_transactions =
        fetch_processed_transactions(second_block_transactions).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions[0].txid()
    );

    assert!(!matches!(
        second_processed_transactions[0].status,
        Status::Failed { .. }
    ));

    let _ = mine_block();

    println!("\x1b[1m\x1B[34m Second Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",second_processed_transactions[0].bitcoin_txid.clone().unwrap());

    thread::sleep(std::time::Duration::from_secs(10));

    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(1, 1));

    assert_eq!(second_account_data, CounterData::new(2, 1));

    //let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    log_scenario_end(
        25,
        &format!(
            "First counter : {:?} / Second counter : {:?} ",
            first_account_data, second_account_data
        ),
    );
}

#[ignore]
#[serial]
#[test]
fn test_read_account_info() {
    // First account : AccountInfoResult { owner: Pubkey([151, 101, 133, 101, 113, 47, 35, 250, 185, 174, 107, 254, 141, 172, 6, 125, 5, 178, 115, 102, 66, 8, 207, 195, 253, 235, 90, 16, 21, 124, 164, 102]), data: [2, 0, 1, 0], utxo: "be1663818e61dd78150973981820ee7cd315f3a09fef5ab35c153583c518c85d:1", is_executable: false, tag: "c4d2b3fd5cfca2f9213e0929624e792880cb02233d4ec0f02055f2d2cf90b834" }
    // Second account : AccountInfoResult { owner: Pubkey([151, 101, 133, 101, 113, 47, 35, 250, 185, 174, 107, 254, 141, 172, 6, 125, 5, 178, 115, 102, 66, 8, 207, 195, 253, 235, 90, 16, 21, 124, 164, 102]), data: [1, 0, 1, 0], utxo: "f42320baa993433c7ab81f089df0a7644f104042614b4fe5222453d0b87609b4:0", is_executable: false, tag: "3744a83e37d34b165da85b64711450c19ea192eadfc647714ce2e228b2b7ccb2" }

    let electrs_client = ElectrsClient::new("http://127.0.0.1:3002".to_string());
    let outspend = electrs_client
        .get_tx_outspend(
            "a539fd23f67ad017009c58c25a0c2d15fb2a67eb9892d05f222fcb30a5dc90eb".to_string(),
            0,
        )
        .unwrap();
    println!("Outspend : {:?}", outspend);

    let outspend = electrs_client
        .get_tx_outspend(
            "cc7840642e90a54b35ede597b696c7eb3739c731e7168e8a59db28c315a60585".to_string(),
            0,
        )
        .unwrap();
    println!("Outspend : {:?}", outspend);
}

#[ignore]
#[serial]
#[test]
fn rbf_reorg() {
    init_logging();

    log_scenario_start(25,
        "2 Counters, same utxo replaced by a greater fee, w/ orphan arch tx",
        "Roll Back scenario : First account updated with utxo, then updated again without anchoring. Sane utxo is then used to update another account in RBF"
    );

    connect_nodes();

    let program_pubkey = try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "E2E-Counter").unwrap();

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    print_title("Second Counter Initialization and increase", 5);

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1).unwrap();

    let anchoring = generate_anchoring_psbt(&account_pubkey);

    let btc_block_hash = mine_block();

    println!();
    print_title(
        &format!(
            "⛏️    Mined a new BTC Block on Regtest : {}    ⛏️  ",
            btc_block_hash.as_raw_hash()
        ),
        4,
    );
    println!();

    print_title("Increasing the first counter using the unique utxo", 5);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        Some(2500),
    );

    let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

    let block_transactions = build_and_send_block(vec![transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    println!(
        "First increase processed transaction id : {}",
        processed_transactions[0].txid()
    );

    // println!("First transaction : {:?}", processed_transactions[0]);

    assert!(!matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions[0].bitcoin_txid.is_some());

    println!("\x1b[1m\x1B[34m First Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",processed_transactions[0].bitcoin_txid.clone().unwrap());

    let first_account_data = get_account_counter(&account_pubkey).unwrap();
    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    assert_eq!(first_account_data, CounterData::new(2, 1));
    assert_eq!(second_account_data, CounterData::new(1, 1));
    println!("First account data : {:?}", first_account_data);
    println!("Second account data : {:?}", second_account_data);

    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string(),
    );
    let rpc_node1 = Client::new(BITCOIN_NODE1_ADDRESS, userpass.clone())
        .expect("rpc shouldn not fail to be initiated");
    let rpc_node2 = Client::new(BITCOIN_NODE2_ADDRESS, userpass.clone())
        .expect("rpc shouldn not fail to be initiated");

    let first_txid =
        Txid::from_str(&processed_transactions[0].bitcoin_txid.clone().unwrap()).unwrap();

    let first_tx = rpc_node1.get_raw_transaction(&first_txid, None).unwrap();
    rpc_node2.send_raw_transaction(&first_tx).unwrap();

    isolate_nodes();

    print_title(
        "Increasing the second counter using the same unique utxo",
        5,
    );

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &second_account_pubkey,
        false,
        false,
        Some((anchoring.0, anchoring.1, false)),
        None,
    );

    let second_transaction = build_transaction(
        vec![second_account_keypair],
        vec![second_increase_istruction],
    );

    let second_block_transactions = build_and_send_block(vec![second_transaction]);

    let second_processed_transactions =
        fetch_processed_transactions(second_block_transactions).unwrap();

    println!(
        "Second increase processed transaction id : {}",
        second_processed_transactions[0].txid()
    );

    assert!(!matches!(
        second_processed_transactions[0].status,
        Status::Failed { .. }
    ));

    println!("\x1b[1m\x1B[34m Second Bitcoin transaction submitted :  : https://mempool.space/testnet4/tx/{} \x1b[0m",second_processed_transactions[0].bitcoin_txid.clone().unwrap());

    thread::sleep(Duration::from_secs(5));
    let first_account_data = get_account_counter(&account_pubkey).unwrap();

    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    println!(
        "First account data : {:?}",
        read_account_info(NODE1_ADDRESS, account_pubkey).unwrap()
    );
    println!(
        "Second account data : {:?}",
        read_account_info(NODE1_ADDRESS, second_account_pubkey).unwrap()
    );
    assert_eq!(first_account_data, CounterData::new(1, 1));
    assert_eq!(second_account_data, CounterData::new(2, 1));

    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string(),
    );
    let rpc_node2 = Client::new(BITCOIN_NODE2_ADDRESS, userpass.clone())
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

    print_title("Increasing the first counter again without anchoring", 5);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        None,
        None,
    );

    let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

    let block_transactions = build_and_send_block(vec![transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    println!(
        "Second increase for first account processed transaction id : {}",
        processed_transactions[0].txid()
    );

    assert!(!matches!(
        processed_transactions[0].status,
        Status::Failed { .. }
    ));

    assert!(processed_transactions[0].bitcoin_txid.is_none());

    thread::sleep(Duration::from_secs(5));
    let first_account_data = get_account_counter(&account_pubkey).unwrap();
    let second_account_data = get_account_counter(&second_account_pubkey).unwrap();

    println!(
        "First account : {:?}",
        read_account_info(NODE1_ADDRESS, account_pubkey).unwrap()
    );
    println!(
        "Second account : {:?}",
        read_account_info(NODE1_ADDRESS, second_account_pubkey).unwrap()
    );

    assert_eq!(first_account_data, CounterData::new(3, 1));
    assert_eq!(second_account_data, CounterData::new(1, 1));

    log_scenario_end(
        25,
        &format!(
            "First counter : {:?} / Second counter : {:?} ",
            first_account_data, second_account_data
        ),
    );
}
