#[cfg(test)]
mod tests {
    use arch_program::{
        account::MIN_ACCOUNT_LAMPORTS,
        sanitized::ArchMessage,
        stake::{
            self,
            program::STAKE_PROGRAM_ID,
            state::{Authorized, Delegation, StakeAuthorize, StakeState},
        },
        system_instruction,
        vote::{program::VOTE_PROGRAM_ID, state::VoteState},
    };
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient};
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, NODE1_ADDRESS},
        helper::{
            create_and_fund_account_with_faucet, read_account_info, send_transactions_and_wait,
        },
        logging::{init_logging, log_scenario_end, log_scenario_start},
    };
    use serial_test::serial;

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_initialize() {
        init_logging();

        log_scenario_start(
            1,
            "Stake Account Initialization",
            "Happy Path Scenario : creating and initializing the stake account",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &stake::instruction::create_account(
                    &user_pubkey,
                    &stake_pubkey,
                    &Authorized::auto(&authority_pubkey),
                    MIN_ACCOUNT_LAMPORTS,
                ),
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let block_transactions = send_transactions_and_wait(vec![tx]);
        let processed_tx = block_transactions[0].clone();

        println!("Processed tx: {:?}", processed_tx);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized::auto(&authority_pubkey))
        );

        log_scenario_end(1, "");
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_authorize() {
        init_logging();

        log_scenario_start(
            1,
            "Stake Account Authorization",
            "Happy Path Scenario : creating and initializing the stake account then authorizing the stake account",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, new_stake_authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, new_withdraw_authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let tx1 = build_and_sign_transaction(
            ArchMessage::new(
                &stake::instruction::create_account(
                    &user_pubkey,
                    &stake_pubkey,
                    &Authorized::auto(&authority_pubkey),
                    MIN_ACCOUNT_LAMPORTS,
                ),
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let tx2 = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::authorize(
                    &stake_pubkey,
                    &authority_pubkey,
                    &new_stake_authority_pubkey,
                    StakeAuthorize::Staker,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx1, tx2]);

        println!("Processed tx: {:?}", processed_txs[0]);
        println!("Processed tx: {:?}", processed_txs[1]);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized {
                staker: new_stake_authority_pubkey,
                withdrawer: authority_pubkey,
            })
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::authorize(
                    &stake_pubkey,
                    &authority_pubkey,
                    &new_withdraw_authority_pubkey,
                    StakeAuthorize::Withdrawer,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        println!("Processed tx: {:?}", processed_txs[0]);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized {
                staker: new_stake_authority_pubkey,
                withdrawer: new_withdraw_authority_pubkey,
            })
        );

        log_scenario_end(1, "");
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_delegate() {
        init_logging();

        log_scenario_start(
            1,
            "Stake Account Delegate",
            "Happy Path Scenario : creating and initializing the stake account then delegating the stake account",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&stake_keypair, BITCOIN_NETWORK);
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let stake_account = read_account_info(stake_pubkey);
        let initial_lamports = stake_account.lamports;

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[
                    system_instruction::allocate(&stake_pubkey, StakeState::size_of() as u64),
                    system_instruction::assign(&stake_pubkey, &STAKE_PROGRAM_ID),
                    stake::instruction::initialize(
                        &stake_pubkey,
                        &Authorized::auto(&authority_pubkey),
                    ),
                    system_instruction::create_account(
                        &user_pubkey,
                        &vote_pubkey,
                        MIN_ACCOUNT_LAMPORTS,
                        VoteState::size_of_new() as u64,
                        &VOTE_PROGRAM_ID,
                    ),
                ],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair, vote_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        println!("Processed tx: {:?}", processed_txs[0]);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized::auto(&authority_pubkey))
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::delegate_stake(
                    &stake_pubkey,
                    &authority_pubkey,
                    &vote_pubkey,
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

        for log in processed_txs[0].logs.iter() {
            println!("{:?}", log);
        }

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Stake(
                Authorized::auto(&authority_pubkey),
                Delegation {
                    voter_pubkey: vote_pubkey,
                    stake: initial_lamports,
                    activation_epoch: 1,
                    deactivation_epoch: u64::MAX,
                },
            )
        );

        log_scenario_end(1, "");
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_deactivate() {
        init_logging();

        log_scenario_start(
            1,
            "Stake Account Deactivate",
            "Happy Path Scenario : creating and initializing the stake account, delegating the stake account, then deactivating the stake account",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&stake_keypair, BITCOIN_NETWORK);
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let stake_account = read_account_info(stake_pubkey);
        let initial_lamports = stake_account.lamports;

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[
                    system_instruction::allocate(&stake_pubkey, StakeState::size_of() as u64),
                    system_instruction::assign(&stake_pubkey, &STAKE_PROGRAM_ID),
                    stake::instruction::initialize(
                        &stake_pubkey,
                        &Authorized::auto(&authority_pubkey),
                    ),
                    system_instruction::create_account(
                        &user_pubkey,
                        &vote_pubkey,
                        MIN_ACCOUNT_LAMPORTS,
                        VoteState::size_of_new() as u64,
                        &VOTE_PROGRAM_ID,
                    ),
                ],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair, vote_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        println!("Processed tx: {:?}", processed_txs[0]);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized::auto(&authority_pubkey))
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::delegate_stake(
                    &stake_pubkey,
                    &authority_pubkey,
                    &vote_pubkey,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        for log in processed_txs[0].logs.iter() {
            println!("{:?}", log);
        }

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Stake(
                Authorized::auto(&authority_pubkey),
                Delegation {
                    voter_pubkey: vote_pubkey,
                    stake: initial_lamports,
                    activation_epoch: 1,
                    deactivation_epoch: u64::MAX,
                },
            )
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::deactivate_stake(
                    &stake_pubkey,
                    &authority_pubkey,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        for log in processed_txs[0].logs.iter() {
            println!("{:?}", log);
        }
        println!("{:?}", processed_txs[0].status);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Stake(
                Authorized::auto(&authority_pubkey),
                Delegation {
                    voter_pubkey: vote_pubkey,
                    stake: initial_lamports,
                    activation_epoch: 1,
                    deactivation_epoch: 1,
                },
            )
        );

        log_scenario_end(1, "");
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_withdraw() {
        init_logging();

        log_scenario_start(
            1,
            "Stake Account Withdraw",
            "Happy Path Scenario : creating and initializing the stake account, delegating the stake account, then withdrawing the stake account",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&stake_keypair, BITCOIN_NETWORK);
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let stake_account = read_account_info(stake_pubkey);
        let initial_lamports = stake_account.lamports;

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[
                    system_instruction::allocate(&stake_pubkey, StakeState::size_of() as u64),
                    system_instruction::assign(&stake_pubkey, &STAKE_PROGRAM_ID),
                    stake::instruction::initialize(
                        &stake_pubkey,
                        &Authorized::auto(&authority_pubkey),
                    ),
                    system_instruction::create_account(
                        &user_pubkey,
                        &vote_pubkey,
                        MIN_ACCOUNT_LAMPORTS,
                        VoteState::size_of_new() as u64,
                        &VOTE_PROGRAM_ID,
                    ),
                ],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair, vote_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        println!("Processed tx: {:?}", processed_txs[0]);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized::auto(&authority_pubkey))
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::delegate_stake(
                    &stake_pubkey,
                    &authority_pubkey,
                    &vote_pubkey,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![user_keypair, authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_txs =
            send_transactions_and_wait(vec![tx.expect("Failed to build and sign transaction")]);

        for log in processed_txs[0].logs.iter() {
            println!("{:?}", log);
        }

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Stake(
                Authorized::auto(&authority_pubkey),
                Delegation {
                    voter_pubkey: vote_pubkey,
                    stake: initial_lamports,
                    activation_epoch: 1,
                    deactivation_epoch: u64::MAX,
                },
            )
        );

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::deactivate_stake(
                    &stake_pubkey,
                    &authority_pubkey,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        for log in processed_txs[0].logs.iter() {
            println!("{:?}", log);
        }
        println!("{:?}", processed_txs[0].status);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);
        println!("lamports: {}", account_info.lamports);

        assert_eq!(stake_account.delegation().unwrap().deactivation_epoch, 1);

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[stake::instruction::withdraw(
                    &stake_pubkey,
                    &authority_pubkey,
                    &user_pubkey,
                    initial_lamports / 2,
                )],
                Some(user_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_txs = send_transactions_and_wait(vec![tx]);

        for log in processed_txs[0].logs.iter() {
            println!("{:?}", log);
        }
        println!("{:?}", processed_txs[0].status);

        let account_info = read_account_info(stake_pubkey);
        let stake_account =
            bincode::deserialize::<StakeState>(&mut account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);
        println!("lamports: {}", account_info.lamports);

        log_scenario_end(1, "");
    }
}
