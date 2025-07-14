/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::bpf_loader::LoaderState;
    use arch_program::sanitized::ArchMessage;
    use arch_program::{
        account::AccountMeta, instruction::Instruction, pubkey::Pubkey, utxo::UtxoMeta,
    };

    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
    };
    use arch_test_sdk::constants::{BITCOIN_NETWORK, NODE1_ADDRESS};
    use arch_test_sdk::helper::{create_and_fund_account_with_faucet, send_transactions_and_wait};
    use arch_test_sdk::helper::{deploy_program, read_account_info, send_utxo};
    use borsh::{BorshDeserialize, BorshSerialize};

    /// Represents the parameters for running the Hello World process
    #[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
    pub struct HelloWorldParams {
        pub vault_bump_seed: u8,
        pub utxo: UtxoMeta,
    }

    #[ignore]
    #[test]
    fn test_deploy_call() {
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) = with_secret_key_file("pda_program.key").unwrap();

        let (authority_keypair, _, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "PDA Program".to_string(),
            "program/target/deploy/pda_program.so".to_string(),
            program_keypair,
            authority_keypair,
        );

        let (payer_account_keypair, payer_account_pubkey, _) =
            generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&payer_account_keypair, BITCOIN_NETWORK);

        let elf = std::fs::read("program/target/deploy/pda_program.so")
            .expect("elf path should be available");
        assert!(
            read_account_info(program_pubkey).data[LoaderState::program_data_offset()..] == elf
        );

        assert!(read_account_info(program_pubkey).is_executable);

        // ####################################################################################################################

        println!("sending THE transaction");

        let (vault_pda_pubkey, vault_bump_seed) = Pubkey::find_program_address(
            &[b"vault", payer_account_pubkey.as_ref()],
            &program_pubkey,
        );

        println!("vault_pda_pubkey: {:?}", vault_pda_pubkey);

        let (utxo_txid, utxo_vout) = send_utxo(vault_pda_pubkey);
        println!(
            "{}:{} {:?}",
            utxo_txid,
            utxo_vout,
            hex::encode(vault_pda_pubkey.serialize())
        );

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[Instruction {
                    program_id: program_pubkey,
                    accounts: vec![
                        AccountMeta {
                            pubkey: payer_account_pubkey,
                            is_signer: true,
                            is_writable: true,
                        },
                        AccountMeta {
                            pubkey: vault_pda_pubkey,
                            is_signer: false,
                            is_writable: true,
                        },
                        AccountMeta {
                            pubkey: Pubkey::system_program(),
                            is_signer: false,
                            is_writable: false,
                        },
                    ],
                    data: borsh::to_vec(&HelloWorldParams {
                        vault_bump_seed,
                        utxo: UtxoMeta::from(
                            hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
                            utxo_vout,
                        ),
                    })
                    .unwrap(),
                }],
                Some(payer_account_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![payer_account_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let block_transactions = send_transactions_and_wait(vec![transaction]);
        let processed_tx = block_transactions[0].clone();

        println!("processed_tx {:?}", processed_tx);

        let vault_pda_last_state = read_account_info(vault_pda_pubkey);
        println!("{:?}", vault_pda_last_state);
        // assert_eq!(
        //     vault_pda_last_state.utxo,
        //     format!("{}:{}", utxo_txid, utxo_vout)
        // );
    }
}
