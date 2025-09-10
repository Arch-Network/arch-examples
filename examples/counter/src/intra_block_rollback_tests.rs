use arch_program::sanitized::ArchMessage;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Config,
    ProgramDeployer, RollbackStatus, Status,
};
use serial_test::serial;

use crate::{
    counter_helpers::generate_anchoring,
    counter_instructions::{get_counter_increase_instruction, start_new_counter},
    ELF_PATH, PROGRAM_FILE_PATH,
};

#[ignore]
#[serial]
#[test]
fn test_intra_block_tx_cache() {
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

    println!("First Counter Initialization and increase");

    let (account_pubkey, account_keypair) =
        start_new_counter(&program_pubkey, 1, 1, &authority_keypair).unwrap();

    let anchoring = generate_anchoring(&account_pubkey);
    let second_anchoring = generate_anchoring(&account_pubkey);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        None,
    );

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        &authority_pubkey,
        false,
        false,
        Some((
            second_anchoring.0.clone(),
            second_anchoring.1.clone(),
            false,
        )),
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

    let txids = client
        .send_transactions(vec![transaction, second_transaction])
        .unwrap();
    let block_transactions = client.wait_for_processed_transactions(txids).unwrap();

    for processed_tx in block_transactions {
        assert!(matches!(processed_tx.status, Status::Processed));
        assert!(matches!(
            processed_tx.rollback_status,
            RollbackStatus::NotRolledback
        ));
    }
}
