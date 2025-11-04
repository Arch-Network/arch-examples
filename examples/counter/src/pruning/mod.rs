use arch_program::sanitized::ArchMessage;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Config,
    ProgramDeployer, Status,
};
use serial_test::serial;
use tracing::info;

use crate::{
    counter_instructions::start_new_counter,
    pruning::pruning_utils::{get_account_utxo, wait_for_blocks},
    ELF_PATH, PROGRAM_FILE_PATH,
};

pub mod pruning_utils;

#[ignore]
#[serial]
#[test]
fn test_state_block_exists() {
    println!("Block with active references in account index should not be pruned");

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

    let utxo = get_account_utxo(&client, &account_pubkey).unwrap();
    info!("Account utxo: {:?}", utxo);
    let increase_istruction = crate::counter_instructions::get_counter_increase_instruction(
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

    assert!(matches!(processed_transaction.status, Status::Processed));

    wait_for_blocks(&client, 200);

    let transaction = client.get_processed_transaction(&txid).unwrap();
    assert!(transaction.is_some());
    println!("Latest state transaction still exists after 200 blocks !");
}

#[ignore]
#[serial]
#[test]
fn test_state_block_pruned() {
    println!("Block with no active references in account index should be pruned");

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

    let first_increase_istruction = crate::counter_instructions::get_counter_increase_instruction(
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

    let first_txid = client.send_transaction(first_increase_transaction).unwrap();
    let first_processed_transaction = client.wait_for_processed_transaction(&first_txid).unwrap();

    println!(
        "First increase processed transaction id : {:?}",
        first_processed_transaction.txid()
    );

    assert!(matches!(
        first_processed_transaction.status,
        Status::Processed
    ));

    let second_increase_istruction = crate::counter_instructions::get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        None,
        None,
    );

    let second_increase_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let second_txid = client
        .send_transaction(second_increase_transaction)
        .unwrap();
    let second_processed_transaction = client.wait_for_processed_transaction(&second_txid).unwrap();
    assert!(matches!(
        second_processed_transaction.status,
        Status::Processed
    ));
    println!(
        "Second increase processed transaction id : {}",
        second_processed_transaction.txid()
    );

    wait_for_blocks(&client, 200);

    let first_transaction = client.get_processed_transaction(&first_txid);
    assert!(first_transaction.is_err());
    let second_transaction = client.get_processed_transaction(&second_txid).unwrap();
    assert!(second_transaction.is_some());
    println!("Overriden state transaction pruned after 200 blocks !");
}
