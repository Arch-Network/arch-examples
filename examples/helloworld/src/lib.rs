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
    use bitcoin::Transaction;
    use borsh::{BorshDeserialize, BorshSerialize};
    use serial_test::serial;

    use std::fs;

    /// Represents the parameters for running the Hello World process
    #[derive(Clone, BorshSerialize, BorshDeserialize)]
    pub struct HelloWorldParams {
        pub name: String,
        pub tx_hex: Vec<u8>,
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_deploy_hello_world() {
        let program_pubkey = try_deploy_program(
            "program/target/sbf-solana-solana/release/helloworldprogram.so",
            PROGRAM_FILE_PATH,
            "Hello World Program",
        )
        .unwrap();

        let program_account_info = read_account_info(NODE1_ADDRESS, program_pubkey).unwrap();

        let elf = fs::read("program/target/sbf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");

        assert!(program_account_info.data == elf);

        assert!(program_account_info.is_executable);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_deploy_call() {
        let program_pubkey = try_deploy_program(
            "program/target/sbf-solana-solana/release/helloworldprogram.so",
            PROGRAM_FILE_PATH,
            "Hello World Program",
        )
        .unwrap();

        print_title("ACCOUNT CREATION & PROGRAM CALL", 5);

        let (first_account_keypair, first_account_pubkey, _) = generate_new_keypair();

        let (txid, vout) = send_utxo(first_account_pubkey);

        println!(
            "\x1b[32m Step 1/4 Successful :\x1b[0m BTC Transaction for account UTXO successfully sent : https://mempool.dev.aws.archnetwork.xyz/tx/{} -- vout : {}",
            txid, vout
        );

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */
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

        assert!(matches!(processed_tx.status, Status::Processed));

        println!("\x1b[32m Step 2/4 Successful :\x1b[0m Arch Account successfully created",);

        /* ------------------- ASSIGNING OWNERSHIP TO THE PROGRAM ------------------- */
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

        let account_info = read_account_info(NODE1_ADDRESS, first_account_pubkey).unwrap();
        assert_eq!(account_info.owner, program_pubkey);

        println!("\x1b[32m Step 3/4 Successful :\x1b[0m Account ownership successfully assigned to the program",);

        /* ---------- CALLING HELLO WORLD PROGRAM WITH THE CREATED ACCOUNT ---------- */

        let fees_psbt = hex::decode(prepare_fees()).unwrap();

        let (txid, _) = sign_and_send_instruction(
            Instruction {
                program_id: program_pubkey,
                accounts: vec![AccountMeta {
                    pubkey: first_account_pubkey,
                    is_signer: true,
                    is_writable: true,
                }],
                data: borsh::to_vec(&HelloWorldParams {
                    name: "arch".to_string(),
                    tx_hex: fees_psbt,
                })
                .unwrap(),
            },
            vec![first_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");

        let account_info = read_account_info(NODE1_ADDRESS, first_account_pubkey).unwrap();

        assert_eq!(
            String::from_utf8(account_info.data.clone()).unwrap(),
            "Hello arch"
        );

        assert_eq!(
            format!("{}:0", processed_tx.bitcoin_txid.unwrap()),
            account_info.utxo
        );

        println!(
            "\x1b[32m Step 4/4 Successful :\x1b[0m Hello World program call was successful ! ",
        );

        print_title(
            &format!(
                "Hello World example Finished Successfully! Final Account data : {}",
                String::from_utf8(account_info.data.clone()).unwrap()
            ),
            5,
        );
    }

    #[ignore]
    #[test]
    fn test_redeploy_call() {
        let (program_keypair, program_pubkey) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (first_account_keypair, first_account_pubkey, _) = generate_new_keypair();

        let (txid, vout) = send_utxo(program_pubkey);

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

        let (txid, _) = sign_and_send_instruction(
            Instruction {
                program_id: program_pubkey,
                accounts: vec![AccountMeta {
                    pubkey: first_account_pubkey,
                    is_signer: true,
                    is_writable: true,
                }],
                data: borsh::to_vec(&HelloWorldParams {
                    name: "arch".to_string(),
                    tx_hex: hex::decode(prepare_fees()).unwrap(),
                })
                .unwrap(),
            },
            vec![first_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let first_account_state = read_account_info(NODE1_ADDRESS, first_account_pubkey).unwrap();
        println!("{:?}", first_account_state);
        assert_eq!(
            String::from_utf8(first_account_state.data.clone()).unwrap(),
            "Hello arch"
        );
        assert_eq!(first_account_state.utxo, format!("{}:{}", txid, 0));
    }
}
