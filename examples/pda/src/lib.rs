/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::{
        account::AccountMeta, instruction::Instruction, pubkey::Pubkey, system_instruction,
        utxo::UtxoMeta,
    };

    use arch_sdk::constants::*;
    use arch_sdk::helper::*;
    use borsh::{BorshDeserialize, BorshSerialize};

    use std::fs;

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

        let (program_keypair, program_pubkey) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (payer_account_keypair, payer_account_pubkey) =
            with_secret_key_file(".payer_account.json")
                .expect("getting payer account info should not fail");

        // let (vault_pda_account_keypair, vault_pda_account_pubkey) =
        //     with_secret_key_file(".vault_pda_account.json")
        //         .expect("getting vault pda account info should not fail");

        let (txid, vout) = send_utxo(program_pubkey);
        println!(
            "{}:{} {:?}",
            txid,
            vout,
            hex::encode(program_pubkey.serialize())
        );

        let (txid, _) = sign_and_send_instruction(
            system_instruction::create_account(
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

        deploy_program_txs(program_keypair, "program/target/deploy/pda_program.so").unwrap();

        println!("{:?}", ());

        let elf =
            fs::read("program/target/deploy/pda_program.so").expect("elf path should be available");
        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey)
                .unwrap()
                .data
                == elf
        );

        let (txid, _) = sign_and_send_instruction(
            Instruction {
                program_id: Pubkey::system_program(),
                accounts: vec![AccountMeta {
                    pubkey: program_pubkey,
                    is_signer: true,
                    is_writable: true,
                }],
                data: vec![2],
            },
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

        let (txid, _) = sign_and_send_instruction(
            Instruction {
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
            },
            vec![payer_account_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let vault_pda_last_state = read_account_info(NODE1_ADDRESS, vault_pda_pubkey).unwrap();
        println!("{:?}", vault_pda_last_state);
        // assert_eq!(
        //     vault_pda_last_state.utxo,
        //     format!("{}:{}", utxo_txid, utxo_vout)
        // );
    }
}
