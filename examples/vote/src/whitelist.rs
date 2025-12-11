#[cfg(test)]
mod whitelist_tests {
    use crate::shared_validator_state::shared_validator_state_tests::try_to_initialize_shared_validator_account;
    use crate::utils::{get_bootnode_keypair_from_file, try_to_create_and_fund_account};
    use arch_program::bitcoin::key::Keypair;
    use arch_program::hash::Hash;
    use arch_program::vote::instruction::{add_peer_to_whitelist, remove_peer_from_whitelist};
    use arch_program::vote::validator_state::SharedValidatorState;
    use arch_program::{pubkey::Pubkey, sanitized::ArchMessage};
    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Config, Status,
    };
    use serial_test::serial;

    fn add_validator_to_whitelist(
        client: &ArchRpcClient,
        validator_compressed_pubkey: &[u8; 33],
        signing_keypair: &Keypair,
    ) -> (SharedValidatorState, Hash) {
        // Step 1: Get keypair account
        let shared_validator_pubkey = Pubkey::from_slice(&[2; 32]);

        let signing_keypair_compressed_pubkey = signing_keypair.public_key().serialize();

        let signing_keypair_arch_pubkey =
            Pubkey::from_slice(&signing_keypair_compressed_pubkey[1..33]);

        try_to_create_and_fund_account(signing_keypair);

        println!(
            "\x1b[32m Step 1/3 Successful:\x1b[0m Got signing_keypair  and account {:?}",
            signing_keypair_arch_pubkey
        );

        let instruction = add_peer_to_whitelist(
            &shared_validator_pubkey,
            &signing_keypair_compressed_pubkey,
            validator_compressed_pubkey,
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(signing_keypair_arch_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![*signing_keypair],
            Config::localnet().network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();
        println!("\x1b[32m Step 2/3 Successful:\x1b[0m Whitelist addition transaction sent");

        let account_info = client.read_account_info(shared_validator_pubkey).unwrap();

        let shared_validator_state =
            bincode::deserialize::<SharedValidatorState>(account_info.data.as_slice()).unwrap();

        println!(
            "\x1b[32m Step 3/3 Successful:\x1b[0m Resulting Validator Shared state successfully retrieved"
        );

        (shared_validator_state, processed_txs.txid())
    }

    fn remove_validator_from_whitelist(
        client: &ArchRpcClient,
        validator_compressed_pubkey: &[u8; 33],
        signing_keypair: &Keypair,
    ) -> (SharedValidatorState, Hash) {
        // Step 1: Get keypair account
        let shared_validator_pubkey = Pubkey::from_slice(&[2; 32]);

        let signing_keypair_compressed_pubkey = signing_keypair.public_key().serialize();
        let signing_keypair_arch_pubkey =
            Pubkey::from_slice(&signing_keypair_compressed_pubkey[1..33]);

        try_to_create_and_fund_account(signing_keypair);

        println!(
            "\x1b[32m Step 1/3 Successful:\x1b[0m Got signing_keypair  and account {:?}",
            signing_keypair_arch_pubkey
        );

        let instruction = remove_peer_from_whitelist(
            &shared_validator_pubkey,
            &signing_keypair_compressed_pubkey,
            validator_compressed_pubkey,
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(signing_keypair_arch_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![*signing_keypair],
            Config::localnet().network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();
        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("\x1b[32m Step 2/3 Successful:\x1b[0m Whitelist addition transaction sent");

        let account_info = client.read_account_info(shared_validator_pubkey).unwrap();

        let shared_validator_state =
            bincode::deserialize::<SharedValidatorState>(account_info.data.as_slice()).unwrap();

        println!(
            "\x1b[32m Step 3/3 Successful:\x1b[0m Resulting Validator Shared state successfully retrieved"
        );

        (shared_validator_state, processed_txs.txid())
    }
    #[ignore]
    #[serial]
    #[test]
    fn test_add_validator_to_whitelist() {
        println!("Adding Validator to Whitelist",);
        println!("Happy Path Scenario : adding a validator to the whitelist",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);

        let (key_pair, _validator_pubkey, _) = generate_new_keypair(config.network);

        let bootnode_keypair = get_bootnode_keypair_from_file();

        let (resulting_shared_account, _) = add_validator_to_whitelist(
            &client,
            &key_pair.public_key().serialize(),
            &bootnode_keypair,
        );

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&key_pair.public_key().serialize().to_vec()),
            "Validator not found in whitelist"
        );
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_add_multiple_validators_to_whitelist() {
        println!("Adding Multiple Validators to Whitelist",);
        println!("Happy Path Scenario : adding multiple validators to the whitelist",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);

        let bootnode_keypair = get_bootnode_keypair_from_file();
        let (key_pair1, _validator1_pubkey, _) = generate_new_keypair(config.network);
        let (key_pair2, _validator2_pubkey, _) = generate_new_keypair(config.network);
        let (key_pair3, _validator3_pubkey, _) = generate_new_keypair(config.network);

        let _ = add_validator_to_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );

        let _ = add_validator_to_whitelist(
            &client,
            &key_pair2.public_key().serialize(),
            &bootnode_keypair,
        );

        let resulting_shared_account = add_validator_to_whitelist(
            &client,
            &key_pair3.public_key().serialize(),
            &bootnode_keypair,
        )
        .0;

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&key_pair1.public_key().serialize().to_vec()),
            "Validator not found in whitelist"
        );

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&key_pair2.public_key().serialize().to_vec()),
            "Validator not found in whitelist"
        );

        assert!(
            resulting_shared_account
                .whitelist
                .contains(&key_pair3.public_key().serialize().to_vec()),
            "Validator not found in whitelist"
        );
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_adding_same_validator_multiple_times() {
        println!("Adding Same Validator Multiple Times",);
        println!("Happy Path Scenario : adding same validator multiple times",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);

        let (key_pair1, _validator1_pubkey, _) = generate_new_keypair(config.network);

        let bootnode_keypair = get_bootnode_keypair_from_file();
        let (resulting_state_1, arch_txid_1) = add_validator_to_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );

        let (resulting_state_2, arch_txid_2) = add_validator_to_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );

        let processed_transaction_1 = client
            .get_processed_transaction(&arch_txid_1)
            .unwrap()
            .unwrap();
        let processed_transaction_2 = client
            .get_processed_transaction(&arch_txid_2)
            .unwrap()
            .unwrap();

        assert_eq!(processed_transaction_1.status, Status::Processed);
        assert!(matches!(processed_transaction_2.status, Status::Failed(_)));

        assert!(
            resulting_state_1
                .whitelist
                .contains(&key_pair1.public_key().serialize().to_vec()),
            "Validator not found in whitelist"
        );

        assert_eq!(resulting_state_1, resulting_state_2,);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_adding_validator_to_whitelist_with_invalid_bootnode() {
        println!("Adding Validator to Whitelist with Invalid Signing pair ( NOT BOOTNODE )",);
        println!(
            "Happy Path Scenario : adding a validator to the whitelist with an invalid bootnode",
        );

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);

        let (key_pair, _validator_pubkey, _) = generate_new_keypair(config.network);

        let (resulting_shared_account, resulting_tx) =
            add_validator_to_whitelist(&client, &key_pair.public_key().serialize(), &key_pair);

        let processed_transaction = client
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
            .contains(&key_pair.public_key().serialize().to_vec()));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_remove_validator_from_whitelist() {
        println!("Removing Validator from Whitelist",);
        println!("Happy Path Scenario : removing a validator from the whitelist",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);
        let (key_pair, _validator_pubkey, _) = generate_new_keypair(config.network);
        let bootnode_keypair = get_bootnode_keypair_from_file();

        let _ = add_validator_to_whitelist(
            &client,
            &key_pair.public_key().serialize(),
            &bootnode_keypair,
        );
        let (resulting_shared_account, _) = remove_validator_from_whitelist(
            &client,
            &key_pair.public_key().serialize(),
            &bootnode_keypair,
        );

        assert!(
            !resulting_shared_account
                .whitelist
                .contains(&key_pair.public_key().serialize().to_vec()),
            "Validator still found in whitelist"
        );
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_remove_multiple_validators_from_whitelist() {
        println!("Removing Multiple Validators from Whitelist",);
        println!("Happy Path Scenario : removing multiple validators from the whitelist",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);
        let bootnode_keypair = get_bootnode_keypair_from_file();
        let (key_pair1, _validator1_pubkey, _) = generate_new_keypair(config.network);
        let (key_pair2, _validator2_pubkey, _) = generate_new_keypair(config.network);
        let (key_pair3, _validator3_pubkey, _) = generate_new_keypair(config.network);

        let _ = add_validator_to_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );
        let _ = add_validator_to_whitelist(
            &client,
            &key_pair2.public_key().serialize(),
            &bootnode_keypair,
        );
        let _ = add_validator_to_whitelist(
            &client,
            &key_pair3.public_key().serialize(),
            &bootnode_keypair,
        );

        let _ = remove_validator_from_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );
        let _ = remove_validator_from_whitelist(
            &client,
            &key_pair2.public_key().serialize(),
            &bootnode_keypair,
        );
        let resulting_shared_account = remove_validator_from_whitelist(
            &client,
            &key_pair3.public_key().serialize(),
            &bootnode_keypair,
        )
        .0;

        assert!(!resulting_shared_account
            .whitelist
            .contains(&key_pair1.public_key().serialize().to_vec()));
        assert!(!resulting_shared_account
            .whitelist
            .contains(&key_pair2.public_key().serialize().to_vec()));
        assert!(!resulting_shared_account
            .whitelist
            .contains(&key_pair3.public_key().serialize().to_vec()));
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_removing_same_validator_multiple_times() {
        println!("Removing Same Validator Multiple Times",);
        println!("Happy Path Scenario : removing same validator multiple times",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);
        let (key_pair1, _validator1_pubkey, _) = generate_new_keypair(config.network);
        let bootnode_keypair = get_bootnode_keypair_from_file();

        let _ = add_validator_to_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );
        let (resulting_state_1, arch_txid_1) = remove_validator_from_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );
        let (resulting_state_2, arch_txid_2) = remove_validator_from_whitelist(
            &client,
            &key_pair1.public_key().serialize(),
            &bootnode_keypair,
        );

        let processed_transaction_1 = client
            .get_processed_transaction(&arch_txid_1)
            .unwrap()
            .unwrap();
        let processed_transaction_2 = client
            .get_processed_transaction(&arch_txid_2)
            .unwrap()
            .unwrap();

        assert_eq!(processed_transaction_1.status, Status::Processed);
        assert!(matches!(processed_transaction_2.status, Status::Failed(_)));
        assert!(!resulting_state_1
            .whitelist
            .contains(&key_pair1.public_key().serialize().to_vec()));
        assert_eq!(resulting_state_1, resulting_state_2);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_removing_validator_from_whitelist_with_invalid_bootnode() {
        println!("Removing Validator from Whitelist with Invalid Signing pair",);
        println!("Happy Path Scenario : removing a validator with an invalid bootnode",);

        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        try_to_initialize_shared_validator_account(&client);
        let (key_pair, _validator_pubkey, _) = generate_new_keypair(config.network);
        let bootnode_keypair = get_bootnode_keypair_from_file();

        let _ = add_validator_to_whitelist(
            &client,
            &key_pair.public_key().serialize(),
            &bootnode_keypair,
        );
        let (resulting_shared_account, resulting_tx) =
            remove_validator_from_whitelist(&client, &key_pair.public_key().serialize(), &key_pair);

        let processed_transaction = client
            .get_processed_transaction(&resulting_tx)
            .unwrap()
            .unwrap();

        assert!(matches!(processed_transaction.status, Status::Failed(_)));
        assert!(resulting_shared_account
            .whitelist
            .contains(&key_pair.public_key().serialize().to_vec()));
    }
}
