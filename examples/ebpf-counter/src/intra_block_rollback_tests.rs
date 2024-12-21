use arch_sdk::constants::PROGRAM_FILE_PATH;
use serial_test::serial;

use crate::{
    counter_deployment::try_deploy_program,
    counter_helpers::{generate_anchoring, init_logging, log_scenario_start, print_title},
    counter_instructions::{
        build_and_send_block, build_transaction, get_counter_increase_instruction,
        start_new_counter,
    },
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
        let anchoring = generate_anchoring(&account_pubkey);

        let btc_block_hash = mine_block();

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

        let btc_block_hash = mine_block();
    }
}
