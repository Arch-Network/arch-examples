#[cfg(test)]
pub(crate) mod shared_validator_state_tests {
    use arch_program::{
        account::{SHARED_VALIDATOR_DATA_ACCOUNT_ID, SHARED_VALIDATOR_STAGING_ACCOUNT_ID},
        bitcoin::key::Keypair,
        pubkey::Pubkey,
        sanitized::ArchMessage,
        vote::{
            instruction::initialize_shared_validator_account_chunk,
            validator_state::SharedValidatorState,
        },
    };
    use arch_sdk::blocking::ArchRpcClient;
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, Config, Status};

    use crate::utils::get_bootnode_keypair_from_file;

    pub(crate) fn try_to_initialize_shared_validator_account(client: &ArchRpcClient) {
        let shared_validator_account_pubkey = Pubkey(SHARED_VALIDATOR_DATA_ACCOUNT_ID);

        let account_info = client
            .read_account_info(shared_validator_account_pubkey)
            .unwrap();

        match account_info.data.is_empty() {
            false => {
                let _shared_validator_account =
                    bincode::deserialize::<SharedValidatorState>(account_info.data.as_slice())
                        .unwrap();

                println!(
                    "\x1b[33m\x1b[1mShared validator account already exists, skipping initialization ! \x1b[0m"
                );
            }
            true => {
                println!("Shared validator account does not exist, initializing it !");
                let (user_keypair, user_pubkey, _) =
                    generate_new_keypair(Config::localnet().network);
                client
                    .create_and_fund_account_with_faucet(&user_keypair)
                    .unwrap();

                let bootnode_keypair = get_bootnode_keypair_from_file();

                let bootnode_compressed_pubkey = bootnode_keypair.public_key().serialize();
                send_transaction_to_initialize_shared_validator_account(
                    client,
                    &user_keypair,
                    &user_pubkey,
                    &bootnode_compressed_pubkey,
                    &[],
                    &[],
                );
            }
        }
    }

    fn send_transaction_to_initialize_shared_validator_account(
        client: &ArchRpcClient,
        user_keypair: &Keypair,
        user_pubkey: &Pubkey,
        bootnode_pubkey: &[u8; 33],
        serialized_pubkey_package: &[u8],
        whitelist: &[[u8; 33]],
    ) {
        let shared_validator_account_pubkey = Pubkey(SHARED_VALIDATOR_DATA_ACCOUNT_ID);
        let shared_validator_staging_pubkey = Pubkey(SHARED_VALIDATOR_STAGING_ACCOUNT_ID);
        let serialized_state = SharedValidatorState::new(
            bootnode_pubkey.to_vec(),
            serialized_pubkey_package.to_vec(),
            whitelist.iter().map(|pubkey| pubkey.to_vec()).collect(),
        )
        .serialize();
        let initialization_instruction = initialize_shared_validator_account_chunk(
            &shared_validator_staging_pubkey,
            &shared_validator_account_pubkey,
            true,
            true,
            0,
            serialized_state,
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[initialization_instruction],
                Some(*user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![*user_keypair],
            Config::localnet().network,
        )
        .expect("Failed to build and sign transaction");
        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        assert_eq!(processed_txs.status, Status::Processed);

        let account_info = client
            .read_account_info(shared_validator_account_pubkey)
            .unwrap();
        let shared_validator_account =
            bincode::deserialize::<SharedValidatorState>(account_info.data.as_slice()).unwrap();

        assert_eq!(
            shared_validator_account,
            SharedValidatorState::new(
                bootnode_pubkey.to_vec(),
                serialized_pubkey_package.to_vec(),
                whitelist.iter().map(|p| p.to_vec()).collect(),
            )
        );
        println!("Successfully initialized shared validator account !");
    }
}
