/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::{account::AccountMeta, instruction::Instruction, system_instruction};

    use arch_sdk::{generate_new_keypair, with_secret_key_file, Status};
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, PROGRAM_FILE_PATH},
        helper::{
            assign_ownership_to_program, create_account, deploy_program, prepare_fees,
            read_account_info, send_utxo, sign_and_send_instruction,
        },
        logging::print_title,
    };
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
        let program_pubkey = deploy_program(
            "program/target/sbf-solana-solana/release/helloworldprogram.so".to_string(),
            PROGRAM_FILE_PATH.to_string(),
            "Hello World Program".to_string(),
        );

        let program_account_info = read_account_info(program_pubkey);

        let elf = fs::read("program/target/sbf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");

        assert!(program_account_info.data == elf);

        assert!(program_account_info.is_executable);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_deploy_call() {
        let program_pubkey = deploy_program(
            "program/target/sbf-solana-solana/release/helloworldprogram.so".to_string(),
            PROGRAM_FILE_PATH.to_string(),
            "Hello World Program".to_string(),
        );

        print_title("ACCOUNT CREATION & PROGRAM CALL", 5);

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let (first_account_keypair, first_account_pubkey, address) = create_account();

        println!(
            "\x1b[32m Step 1/4 Successful :\x1b[0m BTC Transaction for account UTXO successfully sent : {} ",
            arch_test_sdk::constants::get_explorer_address_url(BITCOIN_NETWORK, &address.to_string())
        );

        println!("\x1b[32m Step 2/4 Successful :\x1b[0m Arch Account successfully created",);

        /* ------------------- ASSIGNING OWNERSHIP TO THE PROGRAM ------------------- */
        assign_ownership_to_program(program_pubkey, first_account_pubkey, first_account_keypair);

        println!("\x1b[32m Step 3/4 Successful :\x1b[0m Account ownership successfully assigned to the program",);

        /* ---------- CALLING HELLO WORLD PROGRAM WITH THE CREATED ACCOUNT ---------- */

        let processed_tx = sign_and_send_instruction(
            vec![Instruction {
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
            }],
            vec![first_account_keypair],
        );

        let account_info = read_account_info(first_account_pubkey);

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
        let program_pubkey = deploy_program(
            "program/target/sbf-solana-solana/release/helloworldprogram.so".to_string(),
            PROGRAM_FILE_PATH.to_string(),
            "Hello World Program".to_string(),
        );

        let (program_keypair, program_pubkey) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (first_account_keypair, first_account_pubkey, _) =
            generate_new_keypair(BITCOIN_NETWORK);

        let (txid, vout) = send_utxo(program_pubkey);

        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::create_account(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                program_pubkey,
            )],
            vec![program_keypair],
        );

        println!("processed_tx {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        deploy_program(
            "program/target/sbf-solana-solana/release/helloworldprogram.so".to_string(),
            PROGRAM_FILE_PATH.to_string(),
            "Hello World Program".to_string(),
        );

        println!("{:?}", ());

        let elf = fs::read("program/target/sbf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");
        assert!(read_account_info(program_pubkey).data == elf);

        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::deploy(program_pubkey)],
            vec![program_keypair],
        );

        println!("processed_tx {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        assert!(read_account_info(program_pubkey).is_executable);

        // ####################################################################################################################

        // retract the program from being executable
        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::retract(program_pubkey)],
            vec![program_keypair],
        );

        println!(
            "retract the program from being executable {:?}",
            processed_tx
        );

        assert!(processed_tx.status == Status::Processed);

        // write 10 bytes to the program
        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::write_bytes(
                read_account_info(program_pubkey).data.len() as u32,
                10,
                vec![5; 10],
                program_pubkey,
            )],
            vec![program_keypair],
        );

        println!("write 10 bytes to the program {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        // deploy the program
        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::deploy(program_pubkey)],
            vec![program_keypair],
        );

        println!("deploy the program {:?}", processed_tx);

        assert!(processed_tx.status == Status::Processed);

        deploy_program(
            "program/target/sbf-solana-solana/release/helloworldprogram.so".to_string(),
            PROGRAM_FILE_PATH.to_string(),
            "Hello World Program".to_string(),
        );

        // assert the program has the correct bytes
        let elf = fs::read("program/target/sbf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");
        assert!(read_account_info(program_pubkey).data == elf);

        // deploy the program again
        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::deploy(program_pubkey)],
            vec![program_keypair],
        );

        println!("processed_tx {:?}", processed_tx);

        assert!(read_account_info(program_pubkey).is_executable);

        // ####################################################################################################################

        let (txid, vout) = send_utxo(first_account_pubkey);
        println!(
            "{}:{} {:?}",
            txid,
            vout,
            hex::encode(first_account_pubkey.serialize())
        );

        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::create_account(
                hex::decode(txid.clone()).unwrap().try_into().unwrap(),
                vout,
                first_account_pubkey,
            )],
            vec![first_account_keypair],
        );

        println!("processed_tx {:?}", processed_tx);

        let mut instruction_data = vec![3];
        instruction_data.extend(program_pubkey.serialize());

        let processed_tx = sign_and_send_instruction(
            vec![system_instruction::assign(
                first_account_pubkey,
                program_pubkey,
            )],
            vec![first_account_keypair],
        );

        println!("processed_tx {:?}", processed_tx);

        assert_eq!(
            read_account_info(first_account_pubkey).owner,
            program_pubkey
        );

        // ####################################################################################################################

        println!("sending THE transaction");

        let processed_tx = sign_and_send_instruction(
            vec![Instruction {
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
            }],
            vec![first_account_keypair],
        );

        println!("processed_tx {:?}", processed_tx);

        let first_account_state = read_account_info(first_account_pubkey);
        println!("{:?}", first_account_state);
        assert_eq!(
            String::from_utf8(first_account_state.data.clone()).unwrap(),
            "Hello arch"
        );
        assert_eq!(first_account_state.utxo, format!("{}:{}", txid, 0));
    }
}
