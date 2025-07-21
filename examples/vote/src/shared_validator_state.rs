#[cfg(test)]
pub(crate) mod shared_validator_state_tests {
    use arch_program::{
        bitcoin::key::Keypair,
        pubkey::Pubkey,
        sanitized::ArchMessage,
        vote::{
            instruction::initialize_shared_validator_account, validator_state::SharedValidatorState,
        },
    };
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Status};
    use arch_test_sdk::{
        constants::BITCOIN_NETWORK,
        helper::{
            create_and_fund_account_with_faucet, read_account_info, send_transactions_and_wait,
        },
    };

    use crate::utils::get_bootnode_keypair_from_file;

    pub(crate) fn try_to_initialize_shared_validator_account(client: &ArchRpcClient) {
        let shared_validator_account_pubkey = Pubkey::from_slice(&[2; 32]);

        let account_info = read_account_info(shared_validator_account_pubkey);

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
                let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
                create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);

                let bootnode_keypair = get_bootnode_keypair_from_file();

                let bootnode_pubkey = bootnode_keypair
                    .public_key()
                    .x_only_public_key()
                    .0
                    .serialize();
                let bootnode_arch_pubkey = Pubkey::from_slice(&bootnode_pubkey);
                send_transaction_to_initialize_shared_validator_account(
                    client,
                    &user_keypair,
                    &user_pubkey,
                    &bootnode_arch_pubkey,
                    &vec![],
                    &vec![],
                );
            }
        }
    }

    fn send_transaction_to_initialize_shared_validator_account(
        client: &ArchRpcClient,
        user_keypair: &Keypair,
        user_pubkey: &Pubkey,
        bootnode_pubkey: &Pubkey,
        serialized_pubkey_package: &Vec<u8>,
        whitelist: &Vec<Pubkey>,
    ) {
        let shared_validator_account_pubkey = Pubkey::from_slice(&[2; 32]);

        let initialization_instruction = initialize_shared_validator_account(
            &shared_validator_account_pubkey,
            bootnode_pubkey,
            serialized_pubkey_package,
            whitelist,
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[initialization_instruction],
                Some(*user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![*user_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        assert_eq!(processed_txs[0].status, Status::Processed);

        let account_info = read_account_info(shared_validator_account_pubkey);
        let shared_validator_account =
            bincode::deserialize::<SharedValidatorState>(account_info.data.as_slice()).unwrap();

        assert_eq!(
            shared_validator_account,
            SharedValidatorState::new(
                *bootnode_pubkey,
                serialized_pubkey_package.to_vec(),
                whitelist.to_vec(),
            )
        );
        println!("Successfully initialized shared validator account !");
    }
}
