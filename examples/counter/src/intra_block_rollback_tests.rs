use arch_program::sanitized::ArchMessage;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
    RollbackStatus, Status,
};
use arch_test_sdk::{
    constants::{BITCOIN_NETWORK, NODE1_ADDRESS, PROGRAM_FILE_PATH},
    helper::{create_and_fund_account_with_faucet, deploy_program, send_transactions_and_wait},
    logging::{init_logging, print_title},
};
use serial_test::serial;

use crate::{
    counter_helpers::generate_anchoring,
    counter_instructions::{get_counter_increase_instruction, start_new_counter},
    ELF_PATH,
};

#[ignore]
#[serial]
#[test]
fn test_intra_block_tx_cache() {
    init_logging();

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

    print_title("First Counter Initialization and increase", 5);

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
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let second_transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[second_increase_istruction],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    );

    let block_transactions = send_transactions_and_wait(vec![transaction, second_transaction]);

    for processed_tx in block_transactions {
        assert!(matches!(processed_tx.status, Status::Processed));
        assert!(matches!(
            processed_tx.rollback_status,
            RollbackStatus::NotRolledback
        ));
    }
}
