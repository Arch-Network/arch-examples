#[cfg(test)]
mod update_pubkey_package_tests {

    use arch_program::{
        account::SHARED_VALIDATOR_DATA_ACCOUNT_ID,
        sanitized::ArchMessage,
        vote::{instruction::update_pubkey_package, validator_state::SharedValidatorState},
    };
    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Config, Status,
    };

    use serial_test::serial;
    #[ignore]
    #[serial]
    #[test]
    fn test_update_pubkey_package() {
        println!("Update pubkey package without authority",);
        println!("Updating the pubkey package outside the epoch transition block",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&user_keypair)
            .unwrap();

        let update_instruction = update_pubkey_package(
            &arch_program::pubkey::Pubkey(SHARED_VALIDATOR_DATA_ACCOUNT_ID),
            &[0, 0, 1],
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[update_instruction],
                Some(user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        matches!(processed_txs.status, Status::Failed(_));

        let shared_validator_data_info = client
            .read_account_info(arch_program::pubkey::Pubkey(
                SHARED_VALIDATOR_DATA_ACCOUNT_ID,
            ))
            .unwrap();

        let shared_validator_state =
            SharedValidatorState::deserialize(&shared_validator_data_info.data);

        println!("Shared validator state: {:?}", shared_validator_state);
    }
}
