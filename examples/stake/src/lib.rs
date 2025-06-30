#[cfg(test)]
pub mod builtin_stake_test;

pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/stake_program.so";

#[cfg(test)]
mod stake_tests {
    use crate::ELF_PATH;
    use arch_program::hash::Hash;
    use arch_program::{
        account::AccountMeta, program_pack::Pack, pubkey::Pubkey, sanitized::ArchMessage,
        utxo::UtxoMeta,
    };
    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
        Status,
    };
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, NODE1_ADDRESS, PROGRAM_FILE_PATH},
        helper::{
            create_and_fund_account_with_faucet, deploy_program, read_account_info,
            send_transactions_and_wait, send_utxo,
        },
        logging::{init_logging, log_scenario_start},
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
            // UTXO for mint account creation
            mint_utxo: UtxoMeta,
            // UTXO for stake account creation
            stake_utxo: UtxoMeta,
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
        init_logging();

        log_scenario_start(
            1,
            "Program Deployment & Stake Program Initialization",
            "Deploying the Stake program",
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (user_keypair, user_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        create_and_fund_account_with_faucet(&user_keypair, BITCOIN_NETWORK);

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let program_pubkey = deploy_program(
            "Stake Program".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            user_keypair,
        );

        // generate mint keypair and transfer utxos to it
        let (mint_keypair, mint_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (mint_txid, mint_vout) = send_utxo(mint_pubkey);

        // find stake account and transfer utxos to it
        let stake_account =
            find_stake_account_address(&user_pubkey, &mint_pubkey, &program_pubkey).0;
        let (stake_txid, stake_vout) = send_utxo(stake_account);

        // create utxo meta for mint and stake account
        let mint_utxo = UtxoMeta::from(
            hex::decode(mint_txid.clone()).unwrap().try_into().unwrap(),
            mint_vout,
        );
        let stake_utxo = UtxoMeta::from(
            hex::decode(stake_txid.clone()).unwrap().try_into().unwrap(),
            stake_vout,
        );

        // initialize ix
        initialize(
            mint_utxo,
            stake_utxo,
            user_pubkey,
            user_keypair,
            mint_keypair,
            stake_account,
            mint_pubkey,
            program_pubkey,
            client.get_best_block_hash().unwrap(),
        );

        // create token accounts
        let user_ata = create_ata(
            user_pubkey,
            user_pubkey,
            user_keypair,
            mint_pubkey,
            client.get_best_block_hash().unwrap(),
        );
        let stake_token_account = create_ata(
            user_pubkey,
            stake_account,
            user_keypair,
            mint_pubkey,
            client.get_best_block_hash().unwrap(),
        );

        // mint tokens
        let mint_amount: u64 = 100;
        mint_to(
            mint_amount,
            mint_pubkey,
            user_ata,
            user_pubkey,
            user_keypair,
            client.get_best_block_hash().unwrap(),
        );

        // stake ix
        stake(
            user_pubkey,
            user_keypair,
            user_ata,
            stake_account,
            stake_token_account,
            mint_pubkey,
            program_pubkey,
            client.get_best_block_hash().unwrap(),
        );

        // unstake ix
        unstake(
            user_pubkey,
            user_keypair,
            stake_account,
            mint_pubkey,
            user_ata,
            stake_token_account,
            program_pubkey,
            client.get_best_block_hash().unwrap(),
        );
    }

    pub fn create_ata(
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

        let (txid, vout) = send_utxo(associated_account_address);

        let accounts: Vec<AccountMeta> = vec![
            AccountMeta::new(funder_address, true),
            AccountMeta::new(associated_account_address, false),
            AccountMeta::new(wallet_address, false),
            AccountMeta::new(token_mint_address, false),
            AccountMeta::new(Pubkey::system_program(), false),
            AccountMeta::new(apl_token::id(), false),
        ];
        let mut data = Vec::with_capacity(36); // 32 bytes for txid + 4 bytes for vout
        data.extend_from_slice(&txid.as_bytes());
        data.extend_from_slice(&vout.to_le_bytes());

        let create_ata_txid = build_and_sign_transaction(
            ArchMessage::new(
                &[arch_program::instruction::Instruction {
                    program_id: apl_associated_token_account::id(),
                    accounts,
                    data,
                }],
                Some(funder_address),
                recent_blockhash,
            ),
            vec![funder_address_keypair],
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![create_ata_txid]);
        assert!(processed_tx[0].status == Status::Processed);

        associated_account_address
    }

    pub fn mint_to(
        mint_amount: u64,
        mint_pubkey: Pubkey,
        user_ata: Pubkey,
        user_pubkey: Pubkey,
        user_keypair: Keypair,
        recent_blockhash: Hash,
    ) {
        let mint_to_txid = build_and_sign_transaction(
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
            BITCOIN_NETWORK,
        );
        let processed_tx = send_transactions_and_wait(vec![mint_to_txid]);
        assert!(processed_tx[0].status == Status::Processed);

        let user_ata_info = read_account_info(user_ata);
        assert_eq!(
            apl_token::state::Account::unpack(&user_ata_info.data)
                .unwrap()
                .amount,
            mint_amount
        );
    }

    pub fn initialize(
        mint_utxo: UtxoMeta,
        stake_utxo: UtxoMeta,
        user_pubkey: Pubkey,
        user_keypair: Keypair,
        mint_keypair: Keypair,
        stake_account: Pubkey,
        mint_pubkey: Pubkey,
        program_pubkey: Pubkey,
        recent_blockhash: Hash,
    ) {
        let serialized_initialize_input = borsh::to_vec(&StakeInstruction::Initialize {
            lockup_duration: 0,
            mint_utxo,
            stake_utxo,
        })
        .unwrap();

        let initialize_stake_txid = build_and_sign_transaction(
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
            BITCOIN_NETWORK,
        );
        let processed_tx = send_transactions_and_wait(vec![initialize_stake_txid]);
        assert!(processed_tx[0].status == Status::Processed);

        // check changes after initialize stake
        let stake_account_info = read_account_info(stake_account);
        assert_eq!(stake_account_info.owner, program_pubkey);
    }

    pub fn stake(
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

        let stake_txid = build_and_sign_transaction(
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
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![stake_txid]);
        assert!(processed_tx[0].status == Status::Processed);

        //check that staked amount was updated
        let stake_info = read_account_info(stake_account);
        let staked_amount = StakeAccount::try_from_slice(&stake_info.data)
            .unwrap()
            .staked_amount;
        assert_eq!(stake_amount, staked_amount);

        // user ata balance should be 0
        let user_ata_info = read_account_info(user_ata);
        let user_ata_balance = apl_token::state::Account::unpack(&user_ata_info.data)
            .unwrap()
            .amount;
        assert_eq!(user_ata_balance, 0);
    }

    pub fn unstake(
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

        let unstake_txid = build_and_sign_transaction(
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
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![unstake_txid]);
        assert!(processed_tx[0].status == Status::Processed);

        //check that staked amount was updated
        let stake_info = read_account_info(stake_account);
        let staked_amount = StakeAccount::try_from_slice(&stake_info.data)
            .unwrap()
            .staked_amount;
        assert_eq!(0, staked_amount);

        // user ata balance should be 0
        let user_ata_info = read_account_info(user_ata);
        let user_ata_balance = apl_token::state::Account::unpack(&user_ata_info.data)
            .unwrap()
            .amount;
        assert_eq!(user_ata_balance, 100);
    }
}
