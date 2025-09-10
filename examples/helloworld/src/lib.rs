/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::bpf_loader::LoaderState;
    use arch_program::rent::minimum_rent;
    use arch_program::sanitized::ArchMessage;
    use arch_program::{account::AccountMeta, instruction::Instruction, system_instruction};

    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, prepare_fees, with_secret_key_file,
        ArchRpcClient, BitcoinHelper, Config, ProgramDeployer, Status,
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
        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        let (program_keypair, _) =
            with_secret_key_file(&".program.json").expect("getting caller info should not fail");

        let (authority_keypair, _, _) = generate_new_keypair(config.network);

        client
            .create_and_fund_account_with_faucet(&authority_keypair)
            .unwrap();

        let deployer = ProgramDeployer::new(&config);

        let program_pubkey = deployer
            .try_deploy_program(
                "Hello World Program".to_string(),
                program_keypair,
                authority_keypair,
                &"program/target/sbpf-solana-solana/release/helloworldprogram.so".to_string(),
            )
            .unwrap();

        let program_account_info = client
            .read_account_info(program_pubkey)
            .expect("read account info should not fail");

        let elf = fs::read("program/target/sbpf-solana-solana/release/helloworldprogram.so")
            .expect("elf path should be available");

        assert!(program_account_info.data[LoaderState::program_data_offset()..] == elf);

        assert!(program_account_info.is_executable);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_deploy_call() {
        let config = Config::localnet();
        let client = ArchRpcClient::new(&config);

        let (program_keypair, _) =
            with_secret_key_file(&".program.json").expect("getting caller info should not fail");

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&authority_keypair)
            .unwrap();

        let deployer = ProgramDeployer::new(&config);

        let program_pubkey = deployer
            .try_deploy_program(
                "Hello World Program".to_string(),
                program_keypair,
                authority_keypair,
                &"program/target/sbpf-solana-solana/release/helloworldprogram.so".to_string(),
            )
            .unwrap();

        // print_title("ACCOUNT CREATION & PROGRAM CALL", 5);
        println!("ACCOUNT CREATION & PROGRAM CALL ");

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let (first_account_keypair, first_account_pubkey, _address) =
            generate_new_keypair(config.network);

        // println!(
        //     "\x1b[32m Step 1/3 Successful :\x1b[0m BTC Transaction for account UTXO successfully sent : {} ",
        //    "https://mempool.dev.aws.archnetwork.xyz/address/http://localhost:9002/"
        // );

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let bitcoin_helper = BitcoinHelper::new(&config);
        let (txid, vout) = bitcoin_helper.send_utxo(first_account_pubkey).unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[system_instruction::create_account_with_anchor(
                    &authority_pubkey,
                    &first_account_pubkey,
                    minimum_rent(0),
                    0,
                    &program_pubkey,
                    hex::decode(txid).unwrap().try_into().unwrap(),
                    vout,
                )],
                Some(authority_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![first_account_keypair, authority_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txids = client.send_transactions(vec![transaction]).unwrap();
        let block_transactions = client.wait_for_processed_transactions(txids).unwrap();

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
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txids = client.send_transactions(vec![transaction]).unwrap();
        let block_transactions = client.wait_for_processed_transactions(txids).unwrap();

        let processed_tx = block_transactions[0].clone();

        assert!(matches!(processed_tx.status, Status::Processed));

        let account_info = client.read_account_info(first_account_pubkey).unwrap();

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

        println!(
            "Hello World example Finished Successfully! Final Account data : {}",
            String::from_utf8(account_info.data.clone()).unwrap()
        );
    }

    #[ignore]
    #[serial]
    #[test]
    fn double_spent_shouldnt_be_possible() {
        let config = Config::localnet();

        let client = ArchRpcClient::new(&config);

        let (program_keypair, _) =
            with_secret_key_file(&".program.json").expect("getting caller info should not fail");

        let (authority_keypair, authority_pubkey) =
            with_secret_key_file(".caller.json").expect("getting caller info should not fail");

        client
            .create_and_fund_account_with_faucet(&authority_keypair)
            .unwrap();

        let deployer = ProgramDeployer::new(&config);

        let _program_pubkey = deployer
            .try_deploy_program(
                "Hello World Program".to_string(),
                program_keypair,
                authority_keypair,
                &"program/target/sbpf-solana-solana/release/helloworldprogram.so".to_string(),
            )
            .unwrap();

        println!("ACCOUNT CREATION & PROGRAM CALL");

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let (first_account_keypair, first_account_pubkey, _address) =
            generate_new_keypair(config.network);
        // create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        // println!(
        //     "\x1b[32m Step 1/3 Successful :\x1b[0m BTC Transaction for account UTXO successfully sent : {} ",
        //    "https://mempool.dev.aws.archnetwork.xyz/address/http://localhost:9002/"
        // );

        /* --------------------- CREATING A HELLO WORLD ACCOUNT --------------------- */

        let bitcoin_helper = BitcoinHelper::new(&config);

        let (_txid, _vout) = bitcoin_helper.send_utxo(first_account_pubkey).unwrap();

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
            config.network,
        )
        .expect("Failed to build and sign transaction");
        println!(
            "Authority pubkey {:?}",
            client.read_account_info(authority_pubkey).unwrap().lamports
        );
        let arch_rpc_client = ArchRpcClient::new(&config);
        let _txids = arch_rpc_client.send_transactions(vec![transaction.clone()]);

        let txids = client.send_transactions(vec![transaction]).unwrap();
        let _block_transactions = client.wait_for_processed_transactions(txids).unwrap();

        println!(
            "Authority pubkey {:?}",
            client.read_account_info(authority_pubkey).unwrap().lamports
        );
        println!(
            "first_account_pubkey {:?}",
            client
                .read_account_info(first_account_pubkey)
                .unwrap()
                .lamports
        );
        let first_account_info = client.read_account_info(first_account_pubkey).unwrap();
        // txn not duplicated
        assert_eq!(first_account_info.lamports, 100000);
    }
}
