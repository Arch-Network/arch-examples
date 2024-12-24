/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::{
        account::AccountMeta, instruction::Instruction, system_instruction::SystemInstruction,
        utxo::UtxoMeta,
    };

    use arch_sdk::constants::*;
    use arch_sdk::helper::*;
    use arch_sdk::processed_transaction::Status;
    use borsh::{BorshDeserialize, BorshSerialize};

    use std::fs;

    /// Represents the parameters for running the Hello World process
    #[derive(Clone, BorshSerialize, BorshDeserialize)]
    pub struct HelloWorldParams {
        pub name: String,
        pub tx_hex: Vec<u8>,
        pub utxo: UtxoMeta,
    }

    #[test]
    fn test_sign_with_random_nonce() {
        let (first_account_keypair, _first_account_pubkey) =
            with_secret_key_file(".first_account.json")
                .expect("getting first account info should not fail");

        let signature1 = sign_message_bip322(
            &first_account_keypair,
            b"helloworld",
            bitcoin::Network::Testnet,
        );
        let signature2 = sign_message_bip322(
            &first_account_keypair,
            b"helloworld",
            bitcoin::Network::Testnet,
        );

        println!("signature1 {:?}", signature1);
        println!("signature2 {:?}", signature2);
        assert_ne!(signature1, signature2);
    }

    #[ignore]
    #[test]
    fn test_deploy_call() {
        println!("{:?}", 10044_u64.to_le_bytes());
        println!("{:?}", 10881_u64.to_le_bytes());

        let (program_keypair, program_pubkey) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (first_account_keypair, first_account_pubkey) =
            with_secret_key_file(".first_account.json")
                .expect("getting first account info should not fail");

        let (second_account_keypair, second_account_pubkey) =
            with_secret_key_file(".second_account.json")
                .expect("getting second account info should not fail");

        let (txid, vout) = send_utxo(program_pubkey);
        println!(
            "{}:{} {:?}",
            txid,
            vout,
            hex::encode(program_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_create_account_instruction(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                program_pubkey,
            ),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        deploy_program_txs(
            program_keypair,
            "program/target/sbf-solana-solana/release/helloworldprogram.so",
        )
        .expect("failed to deploy program");

        println!("{:?}", ());

        let elf = fs::read("program/target/sbf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");

        println!(
            "{:?}",
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .data[..100]
                .to_vec()
        );

        println!("{:?}", elf[..100].to_vec());

        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .data
                == elf
        );

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_deploy_instruction(program_pubkey),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .is_executable
        );

        // ####################################################################################################################

        let (txid, vout) = send_utxo(first_account_pubkey);
        println!(
            "{}:{} {:?}",
            txid,
            vout,
            hex::encode(first_account_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_create_account_instruction(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                first_account_pubkey,
            ),
            vec![first_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let mut instruction_data = vec![3];
        instruction_data.extend(program_pubkey.serialize());

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_assign_ownership_instruction(
                first_account_pubkey,
                program_pubkey,
            ),
            vec![first_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .owner,
            program_pubkey
        );

        // ####################################################################################################################

        println!("sending THE transaction");

        let (utxo_txid, utxo_vout) = send_utxo(second_account_pubkey);
        println!(
            "{}:{} {:?}",
            utxo_txid,
            utxo_vout,
            hex::encode(second_account_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            Instruction {
                program_id: program_pubkey,
                accounts: vec![
                    AccountMeta {
                        pubkey: first_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: second_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                ],
                data: borsh::to_vec(&HelloWorldParams {
                    name: "arch".to_string(),
                    tx_hex: hex::decode(prepare_fees()).unwrap(),
                    utxo: UtxoMeta::from(
                        hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
                        utxo_vout,
                    ),
                })
                .unwrap(),
            },
            vec![first_account_keypair, second_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let first_account_last_state =
            read_account_info(NODE1_ADDRESS, first_account_pubkey).unwrap();
        println!("{:?}", first_account_last_state);
        assert_eq!(
            first_account_last_state.utxo,
            format!("{}:{}", processed_tx.bitcoin_txid.unwrap(), 0)
        );

        let second_account_last_state =
            read_account_info(NODE1_ADDRESS, second_account_pubkey).unwrap();
        println!("{:?}", second_account_last_state);
        assert_eq!(
            second_account_last_state.utxo,
            format!("{}:{}", utxo_txid, utxo_vout)
        );

        // ####################################################################################################################

        println!("sending THE transaction");

        let (utxo_txid, utxo_vout) = send_utxo(second_account_pubkey);
        println!(
            "{}:{} {:?}",
            utxo_txid,
            utxo_vout,
            hex::encode(second_account_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            Instruction {
                program_id: program_pubkey,
                accounts: vec![
                    AccountMeta {
                        pubkey: first_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: second_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                ],
                data: borsh::to_vec(&HelloWorldParams {
                    name: "arch".to_string(),
                    tx_hex: hex::decode(prepare_fees()).unwrap(),
                    utxo: UtxoMeta::from(
                        hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
                        utxo_vout,
                    ),
                })
                .unwrap(),
            },
            vec![first_account_keypair, second_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        println!(
            "{:?}",
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .owner,
            first_account_last_state.owner
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .data,
            first_account_last_state.data
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .utxo,
            first_account_last_state.utxo
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .is_executable,
            first_account_last_state.is_executable
        );

        println!(
            "{:?}",
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .owner,
            second_account_last_state.owner
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .data,
            second_account_last_state.data
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .owner,
            second_account_last_state.owner
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .is_executable,
            second_account_last_state.is_executable
        );
    }

    #[ignore]
    #[test]
    fn test_redeploy_call() {
        let (program_keypair, program_pubkey) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (first_account_keypair, first_account_pubkey) =
            with_secret_key_file(".first_account.json")
                .expect("getting first account info should not fail");

        let (second_account_keypair, second_account_pubkey) =
            with_secret_key_file(".second_account.json")
                .expect("getting second account info should not fail");

        let (txid, vout) = send_utxo(program_pubkey);
        println!(
            "{}:{} {:?}",
            txid,
            vout,
            hex::encode(program_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_create_account_instruction(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                program_pubkey,
            ),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        deploy_program_txs(
            program_keypair,
            "program/target/sbf-solana-solana/release/helloworldprogram.so",
        )
        .expect("failed to deploy program");

        println!("{:?}", ());

        let elf = fs::read("program/target/sbf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");
        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .data
                == elf
        );

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_deploy_instruction(program_pubkey),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .is_executable
        );

        // ####################################################################################################################

        // retract the program from being executable
        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_retract_instruction(program_pubkey),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!(
            "retract the program from being executable {:?}",
            processed_tx
        );

        assert!(processed_tx.status == Status::Processed);

        // write 10 bytes to the program
        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_write_bytes_instruction(
                read_account_info(NODE1_ADDRESS, program_pubkey)
                    .unwrap()
                    .data
                    .len() as u32,
                10,
                vec![5; 10],
                program_pubkey,
            ),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("write 10 bytes to the program {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        // deploy the program
        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_deploy_instruction(program_pubkey),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("deploy the program {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        deploy_program_txs(
            program_keypair,
            "program/target/sbf-solana-solana/release/helloworldprogram.so",
        )
        .expect("failed to deploy program");

        // assert the program has the correct bytes
        let elf = fs::read("program/target/sbf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");
        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .data
                == elf
        );

        // deploy the program again
        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_deploy_instruction(program_pubkey),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .is_executable
        );

        // ####################################################################################################################

        let (txid, vout) = send_utxo(first_account_pubkey);
        println!(
            "{}:{} {:?}",
            txid,
            vout,
            hex::encode(first_account_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_create_account_instruction(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                first_account_pubkey,
            ),
            vec![first_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let mut instruction_data = vec![3];
        instruction_data.extend(program_pubkey.serialize());

        let (txid, _) = sign_and_send_instruction(
            SystemInstruction::new_assign_ownership_instruction(
                first_account_pubkey,
                program_pubkey,
            ),
            vec![first_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .owner,
            program_pubkey
        );

        // ####################################################################################################################

        println!("sending THE transaction");

        let (utxo_txid, utxo_vout) = send_utxo(second_account_pubkey);
        println!(
            "{}:{} {:?}",
            utxo_txid,
            utxo_vout,
            hex::encode(second_account_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            Instruction {
                program_id: program_pubkey,
                accounts: vec![
                    AccountMeta {
                        pubkey: first_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: second_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                ],
                data: borsh::to_vec(&HelloWorldParams {
                    name: "arch".to_string(),
                    tx_hex: hex::decode(prepare_fees()).unwrap(),
                    utxo: UtxoMeta::from(
                        hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
                        utxo_vout,
                    ),
                })
                .unwrap(),
            },
            vec![first_account_keypair, second_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let first_account_last_state =
            read_account_info(NODE1_ADDRESS, first_account_pubkey).unwrap();
        println!("{:?}", first_account_last_state);
        assert_eq!(
            first_account_last_state.utxo,
            format!("{}:{}", processed_tx.bitcoin_txid.unwrap(), 0)
        );

        let second_account_last_state =
            read_account_info(NODE1_ADDRESS, second_account_pubkey).unwrap();
        println!("{:?}", second_account_last_state);
        assert_eq!(
            second_account_last_state.utxo,
            format!("{}:{}", utxo_txid, utxo_vout)
        );

        // ####################################################################################################################

        println!("sending THE transaction");

        let (utxo_txid, utxo_vout) = send_utxo(second_account_pubkey);
        println!(
            "{}:{} {:?}",
            utxo_txid,
            utxo_vout,
            hex::encode(second_account_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            Instruction {
                program_id: program_pubkey,
                accounts: vec![
                    AccountMeta {
                        pubkey: first_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: second_account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                ],
                data: borsh::to_vec(&HelloWorldParams {
                    name: "arch".to_string(),
                    tx_hex: hex::decode(prepare_fees()).unwrap(),
                    utxo: UtxoMeta::from(
                        hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
                        utxo_vout,
                    ),
                })
                .unwrap(),
            },
            vec![first_account_keypair, second_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        println!(
            "{:?}",
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .owner,
            first_account_last_state.owner
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .data,
            first_account_last_state.data
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .utxo,
            first_account_last_state.utxo
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, first_account_pubkey)
                .unwrap()
                .is_executable,
            first_account_last_state.is_executable
        );

        println!(
            "{:?}",
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .owner,
            second_account_last_state.owner
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .data,
            second_account_last_state.data
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .owner,
            second_account_last_state.owner
        );
        assert_eq!(
            read_account_info(NODE1_ADDRESS, second_account_pubkey)
                .unwrap()
                .is_executable,
            second_account_last_state.is_executable
        );
    }
}
