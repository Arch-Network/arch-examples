/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::{
        account::AccountMeta, instruction::Instruction, pubkey::Pubkey, utxo::UtxoMeta,
    };

    use arch_sdk::with_secret_key_file;
    use arch_test_sdk::{
        constants::PROGRAM_FILE_PATH,
        helper::{deploy_program, read_account_info, send_utxo, sign_and_send_instruction},
    };
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
        println!("{:?}", 10044_u64.to_le_bytes());
        println!("{:?}", 10881_u64.to_le_bytes());

        let program_pubkey = deploy_program(
            "program/target/deploy/pda_program.so".to_string(),
            PROGRAM_FILE_PATH.to_string(),
            "PDA Program".to_string(),
        );

        let (payer_account_keypair, payer_account_pubkey) =
            with_secret_key_file(".payer_account.json")
                .expect("getting payer account info should not fail");

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

        // let vault_seeds = &[
        //     b"vault",
        //     payer_account_pubkey.as_ref(),
        //     &[vault_bump_seed]
        // ];

        let processed_tx = sign_and_send_instruction(
            vec![Instruction {
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
            vec![payer_account_keypair],
        );

        println!("processed_tx {:?}", processed_tx);

        let vault_pda_last_state = read_account_info(vault_pda_pubkey);
        println!("{:?}", vault_pda_last_state);
        // assert_eq!(
        //     vault_pda_last_state.utxo,
        //     format!("{}:{}", utxo_txid, utxo_vout)
        // );
    }
}
