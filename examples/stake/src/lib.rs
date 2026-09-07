#[cfg(test)]
pub mod builtin_stake_test;

pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/stake_program.so";

#[cfg(test)]
mod stake_tests {
    use crate::ELF_PATH;
    use arch_program::hash::Hash;
    use arch_program::{
        account::AccountMeta, program_pack::Pack, pubkey::Pubkey, sanitized::ArchMessage,
    };
    use arch_sdk::blocking::{ArchRpcClient, ProgramDeployer};
    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, with_secret_key_file, Config, Status,
    };
    use bitcoin::key::Keypair;
    use borsh::{BorshDeserialize, BorshSerialize};
    // Define our instruction types
    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    pub enum StakeInstruction {
        // Initialize a new stake account
        Initialize {
            // Minimum time tokens must be staked
            lockup_duration: u64,
        },
        // Stake tokens
        Stake {
            // Amount of tokens to stake
            amount: u64,
        },
        // Unstake tokens
        Unstake {
            // Amount of tokens to unstake
            amount: u64,
        },
        // Claim rewards
        ClaimRewards,
    }

    // Define the state of our stake account
    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    pub struct StakeAccount {
        // The owner of this stake account
        pub owner: Pubkey,
        // The token mint that this stake account accepts
        pub token_mint: Pubkey,
        // The amount of tokens staked
        pub staked_amount: u64,
        // Timestamp when the stake was created
        pub stake_timestamp: u64,
        // Minimum time tokens must be staked (in seconds)
        pub lockup_duration: u64,
        // Accumulated rewards
        pub rewards: u64,
    }

    // Find the stake account PDA for a given owner and token mint
    pub fn find_stake_account_address(
        owner: &Pubkey,
        token_mint: &Pubkey,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"stake", owner.as_ref(), token_mint.as_ref()], program_id)
    }

    #[ignore]
    #[test]
    pub fn stake_test() {
        let config = Config::localnet();

        println!("Program Deployment & Stake Program Initialization",);
        println!("Deploying the Stake program",);

        let client = ArchRpcClient::new(&config);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(config.network);

        client
            .create_and_fund_program_authority_with_faucet(&user_keypair)
            .unwrap();

        let (program_keypair, _) =
            with_secret_key_file(".program.json").expect("getting caller info should not fail");

        let deployer = ProgramDeployer::new(&config);

        let program_pubkey = deployer
            .try_deploy_program(
                "Stake Program".to_string(),
                program_keypair,
                user_keypair,
                &ELF_PATH.to_string(),
            )
            .unwrap();

        let (mint_keypair, mint_pubkey, _) = generate_new_keypair(config.network);
        let stake_account =
            find_stake_account_address(&user_pubkey, &mint_pubkey, &program_pubkey).0;

        // initialize ix
        initialize(
            &client,
            config.network,
            user_pubkey,
            user_keypair,
            mint_keypair,
            stake_account,
            mint_pubkey,
            program_pubkey,
            client.get_best_finalized_block_hash().unwrap(),
        );

        // create token accounts
        let user_ata = create_ata(
            &client,
            config.network,
            user_pubkey,
            user_pubkey,
            user_keypair,
            mint_pubkey,
            client.get_best_finalized_block_hash().unwrap(),
        );
        let stake_token_account = create_ata(
            &client,
            config.network,
            user_pubkey,
            stake_account,
            user_keypair,
            mint_pubkey,
            client.get_best_finalized_block_hash().unwrap(),
        );

        // mint tokens
        let mint_amount: u64 = 100;
        mint_to(
            &client,
            config.network,
            mint_amount,
            mint_pubkey,
            user_ata,
            user_pubkey,
            user_keypair,
            client.get_best_finalized_block_hash().unwrap(),
        );

        // stake ix
        stake(
            &client,
            config.network,
            user_pubkey,
            user_keypair,
            user_ata,
            stake_account,
            stake_token_account,
            mint_pubkey,
            program_pubkey,
            client.get_best_finalized_block_hash().unwrap(),
        );

        // unstake ix
        unstake(
            &client,
            config.network,
            user_pubkey,
            user_keypair,
            stake_account,
            mint_pubkey,
            user_ata,
            stake_token_account,
            program_pubkey,
            client.get_best_finalized_block_hash().unwrap(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_ata(
        client: &ArchRpcClient,
        bitcoin_network: bitcoin::Network,
        funder_address: Pubkey,
        wallet_address: Pubkey,
        funder_address_keypair: Keypair,
        token_mint_address: Pubkey,
        recent_blockhash: Hash,
    ) -> Pubkey {
        let associated_account_address =
            apl_associated_token_account::get_associated_token_address_and_bump_seed(
                &wallet_address,
                &token_mint_address,
                &apl_associated_token_account::id(),
            )
            .0;

        let create_ata_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[
                    apl_associated_token_account::create_associated_token_account(
                        &funder_address,
                        &associated_account_address,
                        &wallet_address,
                        &token_mint_address,
                        &apl_token::id(),
                        &Pubkey::system_program(),
                    ),
                ],
                Some(funder_address),
                recent_blockhash,
            ),
            vec![funder_address_keypair],
            bitcoin_network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(create_ata_tx).unwrap();
        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();

        assert!(processed_tx.status == Status::Processed);

        associated_account_address
    }

    #[allow(clippy::too_many_arguments)]
    pub fn mint_to(
        client: &ArchRpcClient,
        bitcoin_network: bitcoin::Network,
        mint_amount: u64,
        mint_pubkey: Pubkey,
        user_ata: Pubkey,
        user_pubkey: Pubkey,
        user_keypair: Keypair,
        recent_blockhash: Hash,
    ) {
        let mint_to_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[apl_token::instruction::mint_to(
                    &apl_token::id(),
                    &mint_pubkey,
                    &user_ata,
                    &user_pubkey,
                    &[&user_pubkey],
                    mint_amount,
                )
                .unwrap()],
                Some(user_pubkey),
                recent_blockhash,
            ),
            vec![user_keypair],
            bitcoin_network,
        )
        .expect("Failed to build and sign transaction");
        let txid = client.send_transaction(mint_to_tx).unwrap();
        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();
        assert!(processed_tx.status == Status::Processed);

        let user_ata_info = client.read_account_info(user_ata).unwrap();
        assert_eq!(
            apl_token::state::Account::unpack(&user_ata_info.data)
                .unwrap()
                .amount,
            mint_amount
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        client: &ArchRpcClient,
        bitcoin_network: bitcoin::Network,
        user_pubkey: Pubkey,
        user_keypair: Keypair,
        mint_keypair: Keypair,
        stake_account: Pubkey,
        mint_pubkey: Pubkey,
        program_pubkey: Pubkey,
        recent_blockhash: Hash,
    ) {
        let serialized_initialize_input =
            borsh::to_vec(&StakeInstruction::Initialize { lockup_duration: 0 }).unwrap();

        let initialize_stake_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[arch_program::instruction::Instruction {
                    program_id: program_pubkey,
                    accounts: vec![
                        AccountMeta::new(user_pubkey, true),
                        AccountMeta::new(stake_account, false),
                        AccountMeta::new(mint_pubkey, true),
                        AccountMeta::new_readonly(apl_token::id(), false),
                        AccountMeta::new_readonly(Pubkey::system_program(), false),
                    ],
                    data: serialized_initialize_input,
                }],
                Some(user_pubkey),
                recent_blockhash,
            ),
            vec![user_keypair, mint_keypair],
            bitcoin_network,
        )
        .expect("Failed to build and sign transaction");
        let txid = client.send_transaction(initialize_stake_tx).unwrap();
        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();
        assert!(processed_tx.status == Status::Processed);

        // check changes after initialize stake
        let stake_account_info = client.read_account_info(stake_account).unwrap();
        assert_eq!(stake_account_info.owner, program_pubkey);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stake(
        client: &ArchRpcClient,
        bitcoin_network: bitcoin::Network,
        user_pubkey: Pubkey,
        user_keypair: Keypair,
        user_ata: Pubkey,
        stake_account: Pubkey,
        stake_token_account: Pubkey,
        mint_pubkey: Pubkey,
        program_pubkey: Pubkey,
        recent_blockhash: Hash,
    ) {
        let stake_amount = 100;
        let serialized_stake_input = borsh::to_vec(&StakeInstruction::Stake {
            amount: stake_amount,
        })
        .unwrap();

        let stake_ix_accounts = vec![
            AccountMeta::new(user_pubkey, true),
            AccountMeta::new(stake_account, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new(stake_token_account, false),
            AccountMeta::new(apl_token::id(), false),
        ];

        let stake_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[arch_program::instruction::Instruction {
                    program_id: program_pubkey,
                    accounts: stake_ix_accounts,
                    data: serialized_stake_input,
                }],
                Some(user_pubkey),
                recent_blockhash,
            ),
            vec![user_keypair],
            bitcoin_network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(stake_tx).unwrap();

        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();
        assert!(processed_tx.status == Status::Processed);

        //check that staked amount was updated
        let stake_info = client.read_account_info(stake_account).unwrap();
        let staked_amount = StakeAccount::try_from_slice(&stake_info.data)
            .unwrap()
            .staked_amount;
        assert_eq!(stake_amount, staked_amount);

        // user ata balance should be 0
        let user_ata_info = client.read_account_info(user_ata).unwrap();
        let user_ata_balance = apl_token::state::Account::unpack(&user_ata_info.data)
            .unwrap()
            .amount;
        assert_eq!(user_ata_balance, 0);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn unstake(
        client: &ArchRpcClient,
        bitcoin_network: bitcoin::Network,
        user_pubkey: Pubkey,
        user_keypair: Keypair,
        stake_account: Pubkey,
        mint_pubkey: Pubkey,
        user_ata: Pubkey,
        stake_token_account: Pubkey,
        program_pubkey: Pubkey,
        recent_blockhash: Hash,
    ) {
        let unstake_amount = 100;
        let serialized_unstake_input = borsh::to_vec(&StakeInstruction::Unstake {
            amount: unstake_amount,
        })
        .unwrap();

        let unstake_ix_accounts = vec![
            AccountMeta::new(user_pubkey, true),
            AccountMeta::new(stake_account, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new(stake_token_account, false),
            AccountMeta::new(apl_token::id(), false),
        ];

        let unstake_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[arch_program::instruction::Instruction {
                    program_id: program_pubkey,
                    accounts: unstake_ix_accounts,
                    data: serialized_unstake_input,
                }],
                Some(user_pubkey),
                recent_blockhash,
            ),
            vec![user_keypair],
            bitcoin_network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(unstake_tx).unwrap();

        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();

        assert!(processed_tx.status == Status::Processed);

        //check that staked amount was updated
        let stake_info = client.read_account_info(stake_account).unwrap();
        let staked_amount = StakeAccount::try_from_slice(&stake_info.data)
            .unwrap()
            .staked_amount;
        assert_eq!(0, staked_amount);

        // user ata balance should be 0
        let user_ata_info = client.read_account_info(user_ata).unwrap();
        let user_ata_balance = apl_token::state::Account::unpack(&user_ata_info.data)
            .unwrap()
            .amount;
        assert_eq!(user_ata_balance, 100);
    }
}
