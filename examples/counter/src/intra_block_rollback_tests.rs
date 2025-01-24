use arch_sdk::{
    constants::{NODE1_ADDRESS, PROGRAM_FILE_PATH},
    helper::{
        build_and_send_block, build_transaction, get_processed_transaction, init_logging,
        log_scenario_start, print_title, try_deploy_program,
    },
    processed_transaction::{RollbackStatus, Status},
};
use serial_test::serial;

use crate::{
    counter_helpers::generate_anchoring_psbt,
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

    let program_pubkey = try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "E2E-Counter").unwrap();

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    print_title("Second Counter Initialization and increase", 5);

    let (second_account_pubkey, second_account_keypair) =
        start_new_counter(&program_pubkey, 1, 1).unwrap();

    loop {
        let anchoring = generate_anchoring_psbt(&account_pubkey);

        let _ = mine_block();

        let increase_istruction = get_counter_increase_instruction(
            &program_pubkey,
            &account_pubkey,
            false,
            false,
            Some((anchoring.0.clone(), anchoring.1.clone(), false)),
            Some(2500),
        );

        let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

        let _block_transactions = build_and_send_block(vec![transaction]);

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

        let _second_block_transactions = build_and_send_block(vec![second_transaction]);

        let _ = mine_block();
    }
}

#[ignore]
#[serial]
#[test]
fn test_intra_block_tx_cache() {
    init_logging();

    let program_pubkey = try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "E2E-Counter").unwrap();

    print_title("First Counter Initialization and increase", 5);

    let (account_pubkey, account_keypair) = start_new_counter(&program_pubkey, 1, 1).unwrap();

    let anchoring = generate_anchoring_psbt(&account_pubkey);
    let second_anchoring = generate_anchoring_psbt(&account_pubkey);

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

    let transaction = build_transaction(vec![account_keypair], vec![increase_istruction]);

    let second_transaction =
        build_transaction(vec![account_keypair], vec![second_increase_istruction]);

    let block_transactions = build_and_send_block(vec![transaction, second_transaction]);

    for txid in block_transactions {
        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid).unwrap();
        assert!(matches!(processed_tx.status, Status::Processed));
        assert!(matches!(
            processed_tx.rollback_status,
            RollbackStatus::Rolledback(_)
        ));
    }
}
