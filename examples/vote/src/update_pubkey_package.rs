#[cfg(test)]
mod update_pubkey_package_tests {

    use arch_program::{
        account::SHARED_VALIDATOR_DATA_ACCOUNT_ID,
        sanitized::ArchMessage,
        vote::{instruction::update_pubkey_package, validator_state::SharedValidatorState},
    };
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Status};
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, NODE1_ADDRESS},
        helper::{
            create_and_fund_account_with_faucet, read_account_info, send_transactions_and_wait,
        },
        logging::{init_logging, log_scenario_end, log_scenario_start},
    };
    use serial_test::serial;
    #[ignore]
    #[serial]
    #[test]
    fn test_update_pubkey_package() {
        init_logging();

        log_scenario_start(
            1,
            "Update pubkey package without authority",
            "Updating the pubkey package outside the epoch transition block",
        );

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);

        let update_instruction = update_pubkey_package(
            &arch_program::pubkey::Pubkey(SHARED_VALIDATOR_DATA_ACCOUNT_ID),
            &[0, 0, 1],
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[update_instruction],
                Some(user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        matches!(processed_txs[0].status, Status::Failed(_));

        let shared_validator_data_info = read_account_info(arch_program::pubkey::Pubkey(
            SHARED_VALIDATOR_DATA_ACCOUNT_ID,
        ));

        let shared_validator_state =
            SharedValidatorState::deserialize(&shared_validator_data_info.data);

        println!("Shared validator state: {:?}", shared_validator_state);
        log_scenario_end(1, "");
    }
}
