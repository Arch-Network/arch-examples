/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::account::MIN_ACCOUNT_LAMPORTS;
    use arch_program::bpf_loader::LoaderState;
    use arch_program::sanitized::ArchMessage;
    use arch_program::{account::AccountMeta, instruction::Instruction, system_instruction};

    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
        Status,
    };
    use arch_test_sdk::constants::NODE1_ADDRESS;
    use arch_test_sdk::helper::{create_and_fund_account_with_faucet, send_transactions_and_wait};
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, PROGRAM_FILE_PATH},
        helper::{deploy_program, prepare_fees, read_account_info, send_utxo},
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
        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (authority_keypair, _, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "Hello World Program".to_string(),
            "program/target/sbpf-solana-solana/release/helloworldprogram.so".to_string(),
            program_keypair,
            authority_keypair,
        );

        let program_account_info = read_account_info(program_pubkey);

        let elf = fs::read("program/target/sbpf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");

        assert!(program_account_info.data[LoaderState::program_data_offset()..] == elf);

        assert!(program_account_info.is_executable);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_deploy_call() {
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "Hello World Program".to_string(),
            "program/target/sbpf-solana-solana/release/helloworldprogram.so".to_string(),
            program_keypair,
            authority_keypair,
        );

        print_title("ACCOUNT CREATION & PROGRAM CALL", 5);

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let (first_account_keypair, first_account_pubkey, address) =
            generate_new_keypair(BITCOIN_NETWORK);
        // create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        println!(
            "\x1b[32m Step 1/3 Successful :\x1b[0m BTC Transaction for account UTXO successfully sent : {} ",
            arch_test_sdk::constants::get_explorer_address_url(BITCOIN_NETWORK, &address.to_string())
        );

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let (txid, vout) = send_utxo(first_account_pubkey);

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[system_instruction::create_account_with_anchor(
                    &authority_pubkey,
                    &first_account_pubkey,
                    MIN_ACCOUNT_LAMPORTS,
                    0,
                    &program_pubkey,
                    hex::decode(txid).unwrap().try_into().unwrap(),
                    vout,
                )],
                Some(authority_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![first_account_keypair, authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let block_transactions = send_transactions_and_wait(vec![transaction]);

        let processed_tx = block_transactions[0].clone();

        assert!(matches!(processed_tx.status, Status::Processed));

        println!("\x1b[32m Step 2/3 Successful :\x1b[0m Arch Account successfully created",);

        /* ---------- CALLING HELLO WORLD PROGRAM WITH THE CREATED ACCOUNT ---------- */

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[Instruction {
                    program_id: program_pubkey,
                    accounts: vec![
                        AccountMeta::new(first_account_pubkey, true),
                        AccountMeta::new(authority_pubkey, true),
                    ],
                    data: borsh::to_vec(&HelloWorldParams {
                        name: "arch".to_string(),
                        tx_hex: hex::decode(prepare_fees()).unwrap(),
                    })
                    .unwrap(),
                }],
                Some(authority_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![first_account_keypair, authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let block_transactions = send_transactions_and_wait(vec![transaction]);

        let processed_tx = block_transactions[0].clone();

        assert!(matches!(processed_tx.status, Status::Processed));

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
            "\x1b[32m Step 3/3 Successful :\x1b[0m Hello World program call was successful ! ",
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
    #[serial]
    #[test]
    fn double_spent_shouldnt_be_possible() {
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        // let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (authority_keypair, authority_pubkey) =
            with_secret_key_file(".caller.json").expect("getting caller info should not fail");
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "Hello World Program".to_string(),
            "program/target/sbpf-solana-solana/release/helloworldprogram.so".to_string(),
            program_keypair,
            authority_keypair,
        );

        print_title("ACCOUNT CREATION & PROGRAM CALL", 5);

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let (first_account_keypair, first_account_pubkey, address) =
            generate_new_keypair(BITCOIN_NETWORK);
        // create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        println!(
            "\x1b[32m Step 1/3 Successful :\x1b[0m BTC Transaction for account UTXO successfully sent : {} ",
            arch_test_sdk::constants::get_explorer_address_url(BITCOIN_NETWORK, &address.to_string())
        );

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let (txid, vout) = send_utxo(first_account_pubkey);

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[system_instruction::transfer(
                    &authority_pubkey,
                    &first_account_pubkey,
                    100000,
                )],
                Some(authority_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![first_account_keypair, authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");
        println!(
            "Authority pubkey {:?}",
            read_account_info(authority_pubkey).lamports
        );
        let arch_rpc_client = ArchRpcClient::new(NODE1_ADDRESS);
        let txids = arch_rpc_client.send_transactions(vec![transaction.clone()]);
        let block_transactions: Vec<arch_sdk::ProcessedTransaction> =
            send_transactions_and_wait(vec![transaction]);
        println!(
            "Authority pubkey {:?}",
            read_account_info(authority_pubkey).lamports
        );
        println!(
            "first_account_pubkey {:?}",
            read_account_info(first_account_pubkey).lamports
        );
        let first_account_info = read_account_info(first_account_pubkey);
        // txn not duplicated
        assert_eq!(first_account_info.lamports, 100000);
    }
}
