#[cfg(test)]
mod tests {
    use arch_program::{
        rent::minimum_rent,
        sanitized::ArchMessage,
        stake::{
            self,
            program::STAKE_PROGRAM_ID,
            state::{Authorized, Delegation, StakeAuthorize, StakeState},
        },
        system_instruction,
        vote::{program::VOTE_PROGRAM_ID, state::VoteState},
    };
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Config};
    use serial_test::serial;

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_initialize() {
        let config = Config::localnet();

        println!("Stake Account Initialization",);
        println!("Happy Path Scenario : creating and initializing the stake account",);

        let client = ArchRpcClient::new(&config);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&user_keypair)
            .unwrap();
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(config.network);
        let (_, authority_pubkey, _) = generate_new_keypair(config.network);

        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &stake::instruction::create_account(
                    &user_pubkey,
                    &stake_pubkey,
                    &Authorized::auto(&authority_pubkey),
                    // using 0 lamports here instead of `StakeState::size_of()` as
                    // minimum_rent(0) will be higher anyways
                    // if change that in future, change here too
                    minimum_rent(0),
                ),
                Some(user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let block_transactions = client.wait_for_processed_transaction(&txid).unwrap();
        let processed_tx = block_transactions.clone();

        println!("Processed tx: {:?}", processed_tx);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized::auto(&authority_pubkey))
        );
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_authorize() {
        let config = Config::localnet();

        println!("Stake Account Authorization",);
        println!(
            "Happy Path Scenario : creating and initializing the stake account then authorizing the stake account",
        );

        let client = ArchRpcClient::new(&config);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&user_keypair)
            .unwrap();
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(config.network);
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
        let (_, new_stake_authority_pubkey, _) = generate_new_keypair(config.network);
        let (_, new_withdraw_authority_pubkey, _) = generate_new_keypair(config.network);

        let tx1 = build_and_sign_transaction(
            ArchMessage::new(
                &stake::instruction::create_account(
                    &user_pubkey,
                    &stake_pubkey,
                    &Authorized::auto(&authority_pubkey),
                    minimum_rent(0),
                ),
                Some(user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair],
            config.network,
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txids = client.send_transactions(vec![tx1, tx2]).unwrap();

        let processed_txs = client.wait_for_processed_transactions(txids).unwrap();

        println!("Processed tx: {:?}", processed_txs[0]);
        println!("Processed tx: {:?}", processed_txs[1]);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();
        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("Processed tx: {:?}", processed_txs);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);

        assert_eq!(
            stake_account,
            StakeState::Initialized(Authorized {
                staker: new_stake_authority_pubkey,
                withdrawer: new_withdraw_authority_pubkey,
            })
        );
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_delegate() {
        let config = Config::localnet();

        println!("Stake Account Delegate",);
        println!(
            "Happy Path Scenario : creating and initializing the stake account then delegating the stake account",
        );

        let client = ArchRpcClient::new(&config);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&user_keypair)
            .unwrap();
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&stake_keypair)
            .unwrap();
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(config.network);

        let stake_account = client.read_account_info(stake_pubkey).unwrap();
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
                        minimum_rent(VoteState::size_of_new()),
                        VoteState::size_of_new() as u64,
                        &VOTE_PROGRAM_ID,
                    ),
                ],
                Some(user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair, vote_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("Processed tx: {:?}", processed_txs);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, authority_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("Processed tx: {:?}", processed_txs);

        println!("{:?}", processed_txs.logs);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_deactivate() {
        let config = Config::localnet();

        println!("Stake Account Deactivate",);
        println!(
            "Happy Path Scenario : creating and initializing the stake account, delegating the stake account, then deactivating the stake account",
        );

        let client = ArchRpcClient::new(&config);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&user_keypair)
            .unwrap();
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&stake_keypair)
            .unwrap();
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(config.network);

        let stake_account = client.read_account_info(stake_pubkey).unwrap();
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
                        minimum_rent(VoteState::size_of_new()),
                        VoteState::size_of_new() as u64,
                        &VOTE_PROGRAM_ID,
                    ),
                ],
                Some(user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair, vote_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("Processed tx: {:?}", processed_txs);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, authority_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        for log in processed_txs.logs.iter() {
            println!("{:?}", log);
        }

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();
        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("{:?}", processed_txs.logs);
        println!("{:?}", processed_txs.status);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_stake_withdraw() {
        let config = Config::localnet();

        println!("Stake Account Withdraw",);
        println!(
            "Happy Path Scenario : creating and initializing the stake account, delegating the stake account, then withdrawing the stake account",
        );

        let client = ArchRpcClient::new(&config);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&user_keypair)
            .unwrap();
        let (stake_keypair, stake_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&stake_keypair)
            .unwrap();
        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
        let (vote_keypair, vote_pubkey, _) = generate_new_keypair(config.network);

        let stake_account = client.read_account_info(stake_pubkey).unwrap();
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
                        minimum_rent(VoteState::size_of_new()),
                        VoteState::size_of_new() as u64,
                        &VOTE_PROGRAM_ID,
                    ),
                ],
                Some(user_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, stake_keypair, vote_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("Processed tx: {:?}", processed_txs);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![user_keypair, authority_keypair],
            config.network,
        )
        .unwrap();

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("{:?}", processed_txs.logs);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("{:?}", processed_txs.logs);

        println!("{:?}", processed_txs.status);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
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
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![authority_keypair, user_keypair],
            config.network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(tx).unwrap();

        let processed_txs = client.wait_for_processed_transaction(&txid).unwrap();

        println!("{:?}", processed_txs.logs);
        println!("{:?}", processed_txs.status);

        let account_info = client.read_account_info(stake_pubkey).unwrap();
        let stake_account =
            bincode::deserialize::<StakeState>(account_info.data.as_slice()).unwrap();
        println!("Stake account: {:?}", stake_account);
        println!("lamports: {}", account_info.lamports);
    }
}
