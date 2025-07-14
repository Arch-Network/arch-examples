#[cfg(test)]
mod whitelist_tests {
    use crate::shared_validator_state::shared_validator_state_tests::try_to_initialize_shared_validator_account;
    use crate::utils::{get_bootnode_keypair_from_file, try_to_create_and_fund_account};
    use arch_program::bitcoin::key::Keypair;
    use arch_program::hash::Hash;
    use arch_program::vote::instruction::{add_peer_to_whitelist, remove_peer_from_whitelist};
    use arch_program::vote::validator_state::SharedValidatorState;
    use arch_program::{pubkey::Pubkey, sanitized::ArchMessage};
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Status};
    use arch_test_sdk::constants::{BITCOIN_NETWORK, NODE1_ADDRESS};
    use arch_test_sdk::helper::{read_account_info, send_transactions_and_wait};
    use arch_test_sdk::logging::{init_logging, log_scenario_end, log_scenario_start};
    use serial_test::serial;

    fn add_validator_to_whitelist(
        client: &ArchRpcClient,
        validator_pubkey: &Pubkey,
        signing_keypair: &Keypair,
    ) -> (SharedValidatorState, Hash) {
        // Step 1: Get keypair account
        let shared_validator_pubkey = Pubkey::from_slice(&[2; 32]);

        let signing_keypair_pubkey = signing_keypair
            .public_key()
            .x_only_public_key()
            .0
            .serialize();
        let signing_keypair_arch_pubkey = Pubkey::from_slice(&signing_keypair_pubkey);

        try_to_create_and_fund_account(signing_keypair);

        println!(
            "\x1b[32m Step 1/3 Successful:\x1b[0m Got signing_keypair  and account {:?}",
            signing_keypair_arch_pubkey
        );

        let instruction = add_peer_to_whitelist(
            &shared_validator_pubkey,
            &signing_keypair_arch_pubkey,
            validator_pubkey.clone(),
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(signing_keypair_arch_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![signing_keypair.clone()],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);
        println!("\x1b[32m Step 2/3 Successful:\x1b[0m Whitelist addition transaction sent");

        let account_info = read_account_info(shared_validator_pubkey);

        let shared_validator_state =
            bincode::deserialize::<SharedValidatorState>(&mut account_info.data.as_slice())
                .unwrap();

        println!(
            "\x1b[32m Step 3/3 Successful:\x1b[0m Resulting Validator Shared state successfully retrieved"
        );

        (shared_validator_state, processed_txs[0].txid())
    }

    fn remove_validator_from_whitelist(
        client: &ArchRpcClient,
        validator_pubkey: &Pubkey,
        signing_keypair: &Keypair,
    ) -> (SharedValidatorState, Hash) {
        // Step 1: Get keypair account
        let shared_validator_pubkey = Pubkey::from_slice(&[2; 32]);

        let signing_keypair_pubkey = signing_keypair
            .public_key()
            .x_only_public_key()
            .0
            .serialize();
        let signing_keypair_arch_pubkey = Pubkey::from_slice(&signing_keypair_pubkey);

        try_to_create_and_fund_account(signing_keypair);

        println!(
            "\x1b[32m Step 1/3 Successful:\x1b[0m Got signing_keypair  and account {:?}",
            signing_keypair_arch_pubkey
        );

        let instruction = remove_peer_from_whitelist(
            &shared_validator_pubkey,
            &signing_keypair_arch_pubkey,
            validator_pubkey.clone(),
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(signing_keypair_arch_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![signing_keypair.clone()],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);
        println!("\x1b[32m Step 2/3 Successful:\x1b[0m Whitelist addition transaction sent");

        let account_info = read_account_info(shared_validator_pubkey);

        let shared_validator_state =
            bincode::deserialize::<SharedValidatorState>(&mut account_info.data.as_slice())
                .unwrap();

        println!(
            "\x1b[32m Step 3/3 Successful:\x1b[0m Resulting Validator Shared state successfully retrieved"
        );

        (shared_validator_state, processed_txs[0].txid())
    }
    #[ignore]
    #[serial]
    #[test]
    fn test_add_validator_to_whitelist() {
        init_logging();

        log_scenario_start(
            1,
            "Adding Validator to Whitelist",
            "Happy Path Scenario : adding a validator to the whitelist",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);

        let (_, validator_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let bootnode_keypair = get_bootnode_keypair_from_file();

        let (resulting_shared_account, _) =
            add_validator_to_whitelist(&client, &validator_pubkey, &bootnode_keypair);

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&validator_pubkey),
            "Validator not found in whitelist"
        );

        log_scenario_end(1, &format!("{:?}", resulting_shared_account));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_add_multiple_validators_to_whitelist() {
        init_logging();

        log_scenario_start(
            2,
            "Adding Multiple Validators to Whitelist",
            "Happy Path Scenario : adding multiple validators to the whitelist",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);

        let bootnode_keypair = get_bootnode_keypair_from_file();
        let (_, validator1_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, validator2_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, validator3_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let _ = add_validator_to_whitelist(&client, &validator1_pubkey, &bootnode_keypair);

        let _ = add_validator_to_whitelist(&client, &validator2_pubkey, &bootnode_keypair);

        let resulting_shared_account =
            add_validator_to_whitelist(&client, &validator3_pubkey, &bootnode_keypair).0;

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&validator1_pubkey),
            "Validator not found in whitelist"
        );

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&validator2_pubkey),
            "Validator not found in whitelist"
        );

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&validator3_pubkey),
            "Validator not found in whitelist"
        );

        log_scenario_end(2, &format!("{:?}", resulting_shared_account));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_adding_same_validator_multiple_times() {
        init_logging();

        log_scenario_start(
            3,
            "Adding Same Validator Multiple Times",
            "Happy Path Scenario : adding same validator multiple times",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);

        let (_, validator1_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let bootnode_keypair = get_bootnode_keypair_from_file();
        let (resulting_state_1, arch_txid_1) =
            add_validator_to_whitelist(&client, &validator1_pubkey, &bootnode_keypair);

        let (resulting_state_2, arch_txid_2) =
            add_validator_to_whitelist(&client, &validator1_pubkey, &bootnode_keypair);

        let rpc_client = ArchRpcClient::new(&NODE1_ADDRESS.to_string());

        let processed_transaction_1 = rpc_client
            .get_processed_transaction(&arch_txid_1)
            .unwrap()
            .unwrap();
        let processed_transaction_2 = rpc_client
            .get_processed_transaction(&arch_txid_2)
            .unwrap()
            .unwrap();

        assert_eq!(processed_transaction_1.status, Status::Processed);
        assert!(matches!(processed_transaction_2.status, Status::Failed(_)));

        assert!(
            resulting_state_1.whitelist.contains(&validator1_pubkey),
            "Validator not found in whitelist"
        );

        assert_eq!(resulting_state_1, resulting_state_2,);

        log_scenario_end(3, &format!("{:?}", resulting_state_2));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_adding_validator_to_whitelist_with_invalid_bootnode() {
        init_logging();

        log_scenario_start(
            4,
            "Adding Validator to Whitelist with Invalid Signing pair ( NOT BOOTNODE )",
            "Happy Path Scenario : adding a validator to the whitelist with an invalid bootnode",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);

        let (keypair, validator_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let (resulting_shared_account, resulting_tx) =
            add_validator_to_whitelist(&client, &validator_pubkey, &keypair);

        let rpc_client = ArchRpcClient::new(&NODE1_ADDRESS.to_string());

        let processed_transaction = rpc_client
            .get_processed_transaction(&resulting_tx)
            .unwrap()
            .unwrap();

        assert!(matches!(processed_transaction.status, Status::Failed(_)));
        println!(
            "Processed transaction status: {:?}",
            processed_transaction.status
        );
        assert!(!resulting_shared_account
            .whitelist
            .contains(&validator_pubkey));

        log_scenario_end(4, &format!("{:?}", resulting_shared_account));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_remove_validator_from_whitelist() {
        init_logging();
        log_scenario_start(
            5,
            "Removing Validator from Whitelist",
            "Happy Path Scenario : removing a validator from the whitelist",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);
        let (_, validator_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let bootnode_keypair = get_bootnode_keypair_from_file();

        let _ = add_validator_to_whitelist(&client, &validator_pubkey, &bootnode_keypair);
        let (resulting_shared_account, _) =
            remove_validator_from_whitelist(&client, &validator_pubkey, &bootnode_keypair);

        assert!(
            !resulting_shared_account
                .whitelist
                .contains(&validator_pubkey),
            "Validator still found in whitelist"
        );

        log_scenario_end(5, &format!("{:?}", resulting_shared_account));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_remove_multiple_validators_from_whitelist() {
        init_logging();
        log_scenario_start(
            6,
            "Removing Multiple Validators from Whitelist",
            "Happy Path Scenario : removing multiple validators from the whitelist",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);
        let bootnode_keypair = get_bootnode_keypair_from_file();
        let (_, validator1_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, validator2_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, validator3_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let _ = add_validator_to_whitelist(&client, &validator1_pubkey, &bootnode_keypair);
        let _ = add_validator_to_whitelist(&client, &validator2_pubkey, &bootnode_keypair);
        let _ = add_validator_to_whitelist(&client, &validator3_pubkey, &bootnode_keypair);

        let _ = remove_validator_from_whitelist(&client, &validator1_pubkey, &bootnode_keypair);
        let _ = remove_validator_from_whitelist(&client, &validator2_pubkey, &bootnode_keypair);
        let resulting_shared_account =
            remove_validator_from_whitelist(&client, &validator3_pubkey, &bootnode_keypair).0;

        assert!(!resulting_shared_account
            .whitelist
            .contains(&validator1_pubkey));
        assert!(!resulting_shared_account
            .whitelist
            .contains(&validator2_pubkey));
        assert!(!resulting_shared_account
            .whitelist
            .contains(&validator3_pubkey));

        log_scenario_end(6, &format!("{:?}", resulting_shared_account));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_removing_same_validator_multiple_times() {
        init_logging();
        log_scenario_start(
            7,
            "Removing Same Validator Multiple Times",
            "Happy Path Scenario : removing same validator multiple times",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);
        let (_, validator1_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let bootnode_keypair = get_bootnode_keypair_from_file();

        let _ = add_validator_to_whitelist(&client, &validator1_pubkey, &bootnode_keypair);
        let (resulting_state_1, arch_txid_1) =
            remove_validator_from_whitelist(&client, &validator1_pubkey, &bootnode_keypair);
        let (resulting_state_2, arch_txid_2) =
            remove_validator_from_whitelist(&client, &validator1_pubkey, &bootnode_keypair);

        let rpc_client = ArchRpcClient::new(&NODE1_ADDRESS.to_string());
        let processed_transaction_1 = rpc_client
            .get_processed_transaction(&arch_txid_1)
            .unwrap()
            .unwrap();
        let processed_transaction_2 = rpc_client
            .get_processed_transaction(&arch_txid_2)
            .unwrap()
            .unwrap();

        assert_eq!(processed_transaction_1.status, Status::Processed);
        assert!(matches!(processed_transaction_2.status, Status::Failed(_)));
        assert!(!resulting_state_1.whitelist.contains(&validator1_pubkey));
        assert_eq!(resulting_state_1, resulting_state_2);

        log_scenario_end(7, &format!("{:?}", resulting_state_2));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_removing_validator_from_whitelist_with_invalid_bootnode() {
        init_logging();
        log_scenario_start(
            8,
            "Removing Validator from Whitelist with Invalid Signing pair",
            "Happy Path Scenario : removing a validator with an invalid bootnode",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        try_to_initialize_shared_validator_account(&client);
        let (keypair, validator_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let bootnode_keypair = get_bootnode_keypair_from_file();

        let _ = add_validator_to_whitelist(&client, &validator_pubkey, &bootnode_keypair);
        let (resulting_shared_account, resulting_tx) =
            remove_validator_from_whitelist(&client, &validator_pubkey, &keypair);

        let rpc_client = ArchRpcClient::new(&NODE1_ADDRESS.to_string());
        let processed_transaction = rpc_client
            .get_processed_transaction(&resulting_tx)
            .unwrap()
            .unwrap();

        assert!(matches!(processed_transaction.status, Status::Failed(_)));
        assert!(resulting_shared_account
            .whitelist
            .contains(&validator_pubkey));

        log_scenario_end(8, &format!("{:?}", resulting_shared_account));
    }
}
