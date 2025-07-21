#[cfg(test)]
pub mod secp256k1_signature_tests {
    pub const ELF_PATH: &str =
        "./program/target/sbpf-solana-solana/release/secp256k1_signature_program.so";
    use arch_program::account::AccountMeta;
    use arch_program::instruction::Instruction;
    use arch_program::sanitized::ArchMessage;
    use arch_sdk::build_and_sign_transaction;
    use arch_sdk::generate_new_keypair;
    use arch_sdk::with_secret_key_file;
    use arch_sdk::ArchRpcClient;
    use arch_sdk::Status;
    use arch_test_sdk::constants::BITCOIN_NETWORK;
    use arch_test_sdk::constants::NODE1_ADDRESS;
    use arch_test_sdk::constants::PROGRAM_FILE_PATH;
    use arch_test_sdk::helper::create_and_fund_account_with_faucet;
    use arch_test_sdk::helper::deploy_program;
    use arch_test_sdk::helper::send_transactions_and_wait;
    use arch_test_sdk::logging::init_logging;
    use arch_test_sdk::logging::log_scenario_end;
    use arch_test_sdk::logging::log_scenario_start;
    use borsh::{BorshDeserialize, BorshSerialize};

    use libsecp256k1::sign;
    use libsecp256k1::Message;
    use libsecp256k1::SecretKey;
    use serial_test::serial;
    #[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
    struct Secp256k1Signature {
        pub pubkey: [u8; 64],
        pub signature: [u8; 64],
        pub message_hash: [u8; 32],
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_successful_signature() {
        init_logging();
        log_scenario_start(
            1,
            "Signing a message and verifying the signature within the program",
            "Successful verification of a Secp256k1 signature, provided the message hash, the signature, and the 64-bytes compressed pubkey",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);
        let program_pubkey = deploy_program(
            "Secp256k1-signature".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );

        let (signing_keypair, _, _) = generate_new_keypair(BITCOIN_NETWORK);
        let message_slice = "Message".as_bytes();
        let message_digest = sha256::digest(message_slice);
        let message_hash = hex::decode(message_digest.clone()).unwrap();
        let message: Message = Message::parse_slice(&message_hash).unwrap();
        let secret_key_bytes = signing_keypair.secret_bytes();
        let libsecp256k1_secret_key = SecretKey::parse_slice(&secret_key_bytes).unwrap();
        let (signature, _recovery_id) = sign(&message, &libsecp256k1_secret_key);
        let serialized_message = message.serialize();
        let serialized_signature = signature.serialize();
        let serialized_pubkey_uncompressed = signing_keypair.public_key().serialize_uncompressed();
        let mut serialized_pubkey_compressed = [0u8; 64];
        serialized_pubkey_compressed.copy_from_slice(&serialized_pubkey_uncompressed[1..]);
        println!(
            "Message : {:?} , Signature : {:?}, Pubkey {:?} ",
            serialized_message, serialized_signature, serialized_pubkey_compressed
        );
        let input_signature = Secp256k1Signature {
            pubkey: serialized_pubkey_compressed,
            signature: serialized_signature,
            message_hash: serialized_message,
        };
        let serialized_instruction_data = borsh::to_vec(&input_signature).unwrap();
        let instruction = Instruction {
            program_id: program_pubkey,
            accounts: vec![AccountMeta::new(authority_pubkey, true)],
            data: serialized_instruction_data,
        };
        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");
        let block_transactions = send_transactions_and_wait(vec![transaction]);
        let processed_transaction = block_transactions[0].clone();
        assert!(matches!(processed_transaction.status, Status::Processed));
        log_scenario_end(1, "Verified the signature successfully !");
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_failing_signature() {
        init_logging();
        log_scenario_start(
            2,
            "Verifying an erroneous signature",
            "Failing verification of a Secp256k1 signature, provided the message hash, an erroneous signature, and the 64-bytes compressed pubkey",
        );
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);
        let program_pubkey = deploy_program(
            "Secp256k1-signature".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );
        let (signing_keypair, _, _) = generate_new_keypair(BITCOIN_NETWORK);
        let message_slice = "Message".as_bytes();
        let message_digest = sha256::digest(message_slice);
        let message_hash = hex::decode(message_digest.clone()).unwrap();
        let message: Message = Message::parse_slice(&message_hash).unwrap();
        let secret_key_bytes = signing_keypair.secret_bytes();
        let libsecp256k1_secret_key = SecretKey::parse_slice(&secret_key_bytes).unwrap();
        let (signature, _recovery_id) = sign(&message, &libsecp256k1_secret_key);
        let serialized_message = message.serialize();
        let mut serialized_signature = signature.serialize();
        // Messing up the signature
        serialized_signature[0] += 1;
        let serialized_pubkey_uncompressed = signing_keypair.public_key().serialize_uncompressed();
        let mut serialized_pubkey_compressed = [0u8; 64];
        serialized_pubkey_compressed.copy_from_slice(&serialized_pubkey_uncompressed[1..]);
        println!(
            "Message : {:?} , Signature : {:?}, Pubkey {:?} ",
            serialized_message, serialized_signature, serialized_pubkey_compressed
        );
        let input_signature = Secp256k1Signature {
            pubkey: serialized_pubkey_compressed,
            signature: serialized_signature,
            message_hash: serialized_message,
        };
        let serialized_instruction_data = borsh::to_vec(&input_signature).unwrap();
        let instruction = Instruction {
            program_id: program_pubkey,
            accounts: vec![AccountMeta::new(authority_pubkey, true)],
            data: serialized_instruction_data,
        };
        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");
        let block_transactions = send_transactions_and_wait(vec![transaction]);
        let processed_transaction = block_transactions[0].clone();
        assert!(matches!(processed_transaction.status, Status::Failed(_)));
        log_scenario_end(2, "Program signature verification failed as expected !");
    }
}
