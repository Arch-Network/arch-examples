#[cfg(test)]
pub mod secp256k1_signature_tests {
    pub const ELF_PATH: &str =
        "./program/target/sbf-solana-solana/release/secp256k1_signature_program.so";
    use arch_program::instruction::Instruction;
    use arch_sdk::constants::*;
    use arch_sdk::processed_transaction::Status;
    use borsh::{BorshDeserialize, BorshSerialize};
    use ebpf_counter::counter_deployment::try_deploy_program;
    use ebpf_counter::counter_helpers::generate_new_keypair;
    use ebpf_counter::counter_helpers::{init_logging, log_scenario_end, log_scenario_start};
    use ebpf_counter::counter_instructions::{
        build_and_send_block, build_transaction, fetch_processed_transactions,
    };
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
        let program_pubkey =
            try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Secp256k1-signature").unwrap();
        let (signing_keypair, _, _) = generate_new_keypair();
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
            accounts: vec![],
            data: serialized_instruction_data,
        };
        let transaction = build_transaction(vec![signing_keypair], vec![instruction]);
        let block_transactions = build_and_send_block(vec![transaction]);
        let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();
        assert!(matches!(
            processed_transactions[0].status,
            Status::Processed
        ));
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
        let program_pubkey =
            try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Secp256k1-signature").unwrap();
        let (signing_keypair, _, _) = generate_new_keypair();
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
        serialized_signature[0] = serialized_signature[0] + 1;
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
            accounts: vec![],
            data: serialized_instruction_data,
        };
        let transaction = build_transaction(vec![signing_keypair], vec![instruction]);
        let block_transactions = build_and_send_block(vec![transaction]);
        let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();
        assert!(matches!(
            &processed_transactions[0].status,
            Status::Failed {0: reason }
            if reason.contains("Custom program error:")
        ));
        log_scenario_end(2, "Program signature verification failed as expected !");
    }
}
