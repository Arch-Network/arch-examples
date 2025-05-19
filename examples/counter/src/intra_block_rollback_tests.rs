use arch_sdk::{build_transaction, RollbackStatus, Status};
use arch_test_sdk::{
    constants::{BITCOIN_NETWORK, PROGRAM_FILE_PATH},
    helper::{deploy_program, send_transactions_and_wait},
    logging::{init_logging, log_scenario_start, print_title},
};
use serial_test::serial;

use crate::{
    counter_helpers::generate_anchoring,
    counter_instructions::{get_counter_increase_instruction, start_new_counter},
    rollback_tests::mine_block,
    ELF_PATH,
};

#[ignore]
#[serial]
#[test]
fn test() {
    init_logging();

    log_scenario_start(23,
        "2 Counters, same utxo replaced by a greater fee",
        "Roll Back scenario : Same utxo is used to update different accounts, the replaced transaction should be rolled back"
    );

    let program_pubkey = deploy_program(
        ELF_PATH.to_string(),
        PROGRAM_FILE_PATH.to_string(),
        "E2E-Counter".to_string(),
    );

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    print_title("Second Counter Initialization and increase", 5);

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1).unwrap();

    loop {
        let anchoring = generate_anchoring(&account_pubkey);

        let _ = mine_block();

        let increase_istruction = get_counter_increase_instruction(
            &program_pubkey,
            &account_pubkey,
            false,
            false,
            Some((anchoring.0.clone(), anchoring.1.clone(), false)),
            Some(2500),
        );

        let transaction = build_transaction(
            vec![account_keypair],
            vec![increase_istruction],
            BITCOIN_NETWORK,
        );

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

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
            BITCOIN_NETWORK,
        );

        let second_processed_transactions = send_transactions_and_wait(vec![second_transaction]);

        let _ = mine_block();
    }
}

#[ignore]
#[serial]
#[test]
fn test_intra_block_tx_cache() {
    init_logging();

    let program_pubkey = deploy_program(
        ELF_PATH.to_string(),
        PROGRAM_FILE_PATH.to_string(),
        "E2E-Counter".to_string(),
    );

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    let anchoring = generate_anchoring(&account_pubkey);
    let second_anchoring = generate_anchoring(&account_pubkey);

    let increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        Some((anchoring.0.clone(), anchoring.1.clone(), false)),
        None,
    );

    let second_increase_istruction = get_counter_increase_instruction(
        &program_pubkey,
        &account_pubkey,
        false,
        false,
        Some((
            second_anchoring.0.clone(),
            second_anchoring.1.clone(),
            false,
        )),
        None,
    );

    let transaction = build_transaction(
        vec![account_keypair],
        vec![increase_istruction],
        BITCOIN_NETWORK,
    );

    let second_transaction = build_transaction(
        vec![account_keypair],
        vec![second_increase_istruction],
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
