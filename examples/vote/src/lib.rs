pub(crate) mod shared_validator_state;
mod update_pubkey_package;
pub(crate) mod utils;
mod whitelist;
#[cfg(test)]
mod tests {
    use arch_program::{
        account::MIN_ACCOUNT_LAMPORTS,
        bitcoin::key::Keypair,
        pubkey::Pubkey,
        sanitized::ArchMessage,
        vote::{
            self,
            state::{VoteInit, VoteState},
        },
    };
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Status};
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, NODE1_ADDRESS},
        helper::{
            create_and_fund_account_with_faucet, read_account_info, send_transactions_and_wait,
            try_read_account_info,
        },
        logging::{init_logging, log_scenario_end, log_scenario_start},
    };
    use serial_test::serial;

    use crate::utils::get_peer_keypair_from_file;

    #[ignore]
    #[serial]
    #[test]
    fn test_vote_initialize() {
        init_logging();

        log_scenario_start(
            1,
            "Vote Account Initialization",
            "Happy Path Scenario : creating and initializing the vote account",
        );

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, node_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        initialize_vote_account(
            &client,
            &user_pubkey,
            &user_keypair,
            &vote_pubkey,
            &vote_keypair,
            &node_pubkey,
            &authority_pubkey,
        );
        log_scenario_end(1, "");
    }

    pub(crate) fn initialize_vote_account(
        client: &ArchRpcClient,
        user_pubkey: &Pubkey,
        user_keypair: &Keypair,
        vote_pubkey: &Pubkey,
        vote_keypair: &Keypair,
        node_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
    ) {
        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &vote::instruction::create_account(
                    user_pubkey,
                    vote_pubkey,
                    &VoteInit::new(*node_pubkey, *authority_pubkey, 0),
                    MIN_ACCOUNT_LAMPORTS,
                ),
                Some(*user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![*user_keypair, *vote_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        assert_eq!(processed_txs[0].status, Status::Processed);

        let account_info = read_account_info(*vote_pubkey);
        let vote_account = bincode::deserialize::<VoteState>(account_info.data.as_slice()).unwrap();
        println!("Vote account: {:?}", vote_account);

        assert_eq!(
            vote_account,
            VoteState::new(&VoteInit::new(*node_pubkey, *authority_pubkey, 0))
        );
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_vote_authorize() {
        init_logging();

        log_scenario_start(
            1,
            "Vote Account Authorization",
            "Happy Path Scenario : authorizing the vote account",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, node_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, new_authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        initialize_vote_account(
            &client,
            &user_pubkey,
            &user_keypair,
            &vote_pubkey,
            &vote_keypair,
            &node_pubkey,
            &authority_pubkey,
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[vote::instruction::authorize(
                    &vote_pubkey,
                    &authority_pubkey,
                    &new_authority_pubkey,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        println!("Processed tx: {:?}", processed_txs[0]);

        let account_info = read_account_info(vote_pubkey);
        let vote_account = bincode::deserialize::<VoteState>(account_info.data.as_slice()).unwrap();
        println!("Vote account: {:?}", vote_account);

        assert_eq!(vote_account.authority, new_authority_pubkey);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_vote_update_commission() {
        init_logging();

        log_scenario_start(
            1,
            "Vote Account Update Commission",
            "Happy Path Scenario : updating the commission of the vote account",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, node_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        initialize_vote_account(
            &client,
            &user_pubkey,
            &user_keypair,
            &vote_pubkey,
            &vote_keypair,
            &node_pubkey,
            &authority_pubkey,
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[vote::instruction::update_commission(
                    &vote_pubkey,
                    &authority_pubkey,
                    10,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        println!("Processed tx: {:?}", processed_txs[0]);

        let account_info = read_account_info(vote_pubkey);
        let vote_account = bincode::deserialize::<VoteState>(account_info.data.as_slice()).unwrap();
        println!("Vote account: {:?}", vote_account);

        assert_eq!(vote_account.commission, 10);
    }

    #[ignore]
    #[serial]
    #[test]
    fn try_create_vote_account_for_whitelisted_peer() {
        init_logging();

        log_scenario_start(
            1,
            "Vote Account Creation for a whitelisted peer",
            "Happy Path Scenario : creating and initializing vote account for a whitelisted peer",
        );

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let node_keypair = get_peer_keypair_from_file(0);
        let serialized_node_pubkey = node_keypair.public_key().x_only_public_key().0.serialize();
        let node_pubkey = Pubkey::from_slice(&serialized_node_pubkey);

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        match try_read_account_info(node_pubkey) {
            Some(account_info) => {
                let vote_account =
                    bincode::deserialize::<VoteState>(account_info.data.as_slice()).unwrap();

                println!("Vote state already initialized ! {:?}", vote_account);
            }
            None => {
                println!("Vote state not initialized !");

                initialize_vote_account(
                    &client,
                    &user_pubkey,
                    &user_keypair,
                    &vote_pubkey,
                    &vote_keypair,
                    &node_pubkey,
                    &node_pubkey,
                );

                assert!(try_read_account_info(vote_pubkey).is_some());
            }
        }
    }
}
