pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/escrow_program.so";

/// Running Tests
#[cfg(test)]
mod tests {
    use crate::ELF_PATH;
    use arch_program::{
        account::AccountMeta, program_pack::Pack, pubkey::Pubkey, rent::minimum_rent,
        sanitized::ArchMessage, utxo::UtxoMeta,
    };
    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
        BitcoinHelper, Config, ProgramDeployer, Status,
    };

    use bitcoin::key::Keypair;
    use borsh::{BorshDeserialize, BorshSerialize};

    #[derive(BorshDeserialize, BorshSerialize, Debug)]
    pub struct MakeOffer {
        /// The bump seed for the offer's Program Derived Address
        pub offer_bump_seed: u8,
        /// The UTXO metadata associated with this offer
        pub offer_utxo: UtxoMeta,
        /// Unique identifier for the offer
        pub id: u64,
        /// Amount of token A being offered
        pub token_a_offered_amount: u64,
        /// Amount of token B wanted in exchange
        pub token_b_wanted_amount: u64,
    }

    #[derive(BorshDeserialize, BorshSerialize, Debug)]
    pub struct Offer {
        /// Unique identifier for the offer
        pub id: u64,
        /// Public key of the user who created the offer
        pub maker: Pubkey,
        /// Public key of the mint for token A
        pub token_mint_a: Pubkey,
        /// Public key of the mint for token B
        pub token_mint_b: Pubkey,
        /// Amount of token B wanted in exchange
        pub token_b_wanted_amount: u64,
        /// The bump seed for the offer's Program Derived Address
        pub bump: u8,
    }

    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    enum EscrowInstruction {
        /// Create a new offer to exchange tokens
        MakeOffer(MakeOffer),
        /// Accept an existing offer
        TakeOffer,
    }

    #[ignore]
    #[test]
    fn escrow_test() {
        let config = Config::localnet();

        println!("Program Deployment & Escros Program Initialization",);
        println!("Deploying the Escrow program",);

        let client = ArchRpcClient::new(&config);

        let (maker_keypair, maker_pubkey, _) = generate_new_keypair(config.network);

        client
            .create_and_fund_account_with_faucet(&maker_keypair)
            .unwrap();

        let (program_keypair, _) =
            with_secret_key_file(".program").expect("getting caller info should not fail");

        let deployer = ProgramDeployer::new(&config);

        let program_pubkey = deployer
            .try_deploy_program(
                "Escrow Program".to_string(),
                program_keypair,
                maker_keypair,
                &ELF_PATH.to_string(),
            )
            .unwrap();

        let mint_a = create_mint(&maker_pubkey, maker_keypair, client.clone());
        let mint_b = create_mint(&maker_pubkey, maker_keypair, client.clone());

        let id: u64 = 1;
        let offer_seeds = &[b"offer", maker_pubkey.as_ref(), &id.to_le_bytes()];
        let expected_offer_pda = Pubkey::find_program_address(offer_seeds, &program_pubkey);

        let helper = BitcoinHelper::new(&config);

        let (offer_txid, offer_vout) = helper.send_utxo(expected_offer_pda.0).unwrap();
        let offer_utxo = UtxoMeta::from(
            hex::decode(offer_txid.clone()).unwrap().try_into().unwrap(),
            offer_vout,
        );

        let vault = create_ata(
            maker_pubkey,
            expected_offer_pda.0,
            maker_keypair,
            mint_a,
            client.clone(),
        );

        make_offer(
            maker_pubkey,
            maker_keypair,
            mint_a,
            mint_b,
            vault,
            expected_offer_pda,
            offer_utxo,
            id,
            program_pubkey,
            client.clone(),
        );

        let (taker_keypair, taker_pubkey, _) = generate_new_keypair(config.network);
        client
            .create_and_fund_account_with_faucet(&taker_keypair)
            .unwrap();

        let maker_ata_b = create_ata(
            maker_pubkey,
            maker_pubkey,
            maker_keypair,
            mint_b,
            client.clone(),
        );
        let taker_ata_a = create_ata(
            taker_pubkey,
            taker_pubkey,
            taker_keypair,
            mint_a,
            client.clone(),
        );
        let taker_ata_b = create_ata(
            taker_pubkey,
            taker_pubkey,
            taker_keypair,
            mint_b,
            client.clone(),
        );

        take_offer(
            maker_ata_b,
            taker_ata_a,
            taker_ata_b,
            mint_a,
            mint_b,
            maker_pubkey,
            maker_keypair,
            taker_pubkey,
            taker_keypair,
            vault,
            expected_offer_pda,
            program_pubkey,
            client,
        );
    }

    pub fn create_mint(payer: &Pubkey, payer_keypair: Keypair, client: ArchRpcClient) -> Pubkey {
        let config = Config::localnet();

        let (mint_keypair, mint_pubkey, _) = generate_new_keypair(config.network);
        let helper = BitcoinHelper::new(&config);
        let (mint_txid, mint_vout) = helper.send_utxo(mint_pubkey).unwrap();
        let mint_utxo = UtxoMeta::from(
            hex::decode(mint_txid.clone()).unwrap().try_into().unwrap(),
            mint_vout,
        );

        let message = ArchMessage::new(
            &[
                arch_program::system_instruction::create_account_with_anchor(
                    payer,
                    &mint_pubkey,
                    minimum_rent(apl_token::state::Mint::LEN),
                    apl_token::state::Mint::LEN as u64,
                    &apl_token::id(),
                    mint_utxo.txid().try_into().unwrap(),
                    mint_utxo.vout(),
                ),
            ],
            Some(*payer),
            client.get_best_finalized_block_hash().unwrap(),
        );

        let signers = vec![payer_keypair, mint_keypair];

        let create_account_tx = build_and_sign_transaction(message, signers, config.network)
            .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(create_account_tx).unwrap();

        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();
        assert!(processed_tx.status == Status::Processed);

        let message = ArchMessage::new(
            &[apl_token::instruction::initialize_mint(
                &apl_token::id(),
                &mint_pubkey,
                payer,
                Some(payer),
                9,
            )
            .unwrap()],
            Some(*payer),
            client.get_best_finalized_block_hash().unwrap(),
        );

        let signers = vec![payer_keypair, mint_keypair];

        let initialize_mint_tx = build_and_sign_transaction(message, signers, config.network)
            .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(initialize_mint_tx).unwrap();
        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();
        assert!(processed_tx.status == Status::Processed);

        mint_pubkey
    }

    pub fn create_ata(
        funder_address: Pubkey,
        wallet_address: Pubkey,
        funder_address_keypair: Keypair,
        token_mint_address: Pubkey,
        client: ArchRpcClient,
    ) -> Pubkey {
        let test_config = Config::localnet();
        let bitcoin_network = test_config.network;

        let associated_account_address =
            apl_associated_token_account::get_associated_token_address_and_bump_seed(
                &wallet_address,
                &token_mint_address,
                &apl_associated_token_account::id(),
            )
            .0;

        let helper = BitcoinHelper::new(&test_config);
        let (txid, vout) = helper.send_utxo(associated_account_address).unwrap();

        let accounts: Vec<AccountMeta> = vec![
            AccountMeta::new(funder_address, true),
            AccountMeta::new(associated_account_address, false),
            AccountMeta::new(wallet_address, false),
            AccountMeta::new(token_mint_address, false),
            AccountMeta::new(Pubkey::system_program(), false),
            AccountMeta::new(apl_token::id(), false),
        ];
        let mut data = Vec::with_capacity(36); // 32 bytes for txid + 4 bytes for vout
        data.extend_from_slice(txid.as_bytes());
        data.extend_from_slice(&vout.to_le_bytes());

        let create_ata_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[arch_program::instruction::Instruction {
                    program_id: apl_associated_token_account::id(),
                    accounts,
                    data,
                }],
                Some(funder_address),
                client.get_best_finalized_block_hash().unwrap(),
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

    pub fn make_offer(
        maker_pubkey: Pubkey,
        maker_keypair: Keypair,
        mint_a: Pubkey,
        mint_b: Pubkey,
        vault: Pubkey,
        expected_offer_pda: (Pubkey, u8),
        offer_utxo: UtxoMeta,
        id: u64,
        program_pubkey: Pubkey,
        client: ArchRpcClient,
    ) {
        let test_config = Config::localnet();
        let bitcoin_network = test_config.network;

        let maker_ata_a = create_ata(
            maker_pubkey,
            maker_pubkey,
            maker_keypair,
            mint_a,
            client.clone(),
        );

        mint_to(
            100,
            mint_a,
            maker_ata_a,
            maker_pubkey,
            maker_keypair,
            client.clone(),
        );

        let make_offer = MakeOffer {
            offer_bump_seed: expected_offer_pda.1,
            offer_utxo,
            id,
            token_a_offered_amount: 100,
            token_b_wanted_amount: 100,
        };

        let serialized_maker_offer_input =
            borsh::to_vec(&EscrowInstruction::MakeOffer(make_offer)).unwrap();

        let make_offer_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[arch_program::instruction::Instruction {
                    program_id: program_pubkey,
                    accounts: vec![
                        AccountMeta::new(expected_offer_pda.0, false),
                        AccountMeta::new(mint_a, false),
                        AccountMeta::new(mint_b, false),
                        AccountMeta::new(maker_ata_a, false),
                        AccountMeta::new(vault, false),
                        AccountMeta::new(maker_pubkey, true),
                        AccountMeta::new_readonly(apl_token::id(), false),
                        AccountMeta::new_readonly(Pubkey::system_program(), false),
                        AccountMeta::new_readonly(apl_associated_token_account::id(), false),
                    ],
                    data: serialized_maker_offer_input,
                }],
                Some(maker_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![maker_keypair],
            bitcoin_network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(make_offer_tx).unwrap();
        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();
        assert!(processed_tx.status == Status::Processed);
    }

    pub fn take_offer(
        maker_ata_b: Pubkey,
        taker_ata_a: Pubkey,
        taker_ata_b: Pubkey,
        mint_a: Pubkey,
        mint_b: Pubkey,
        maker_pubkey: Pubkey,
        maker_keypair: Keypair,
        taker_pubkey: Pubkey,
        taker_keypair: Keypair,
        vault: Pubkey,
        expected_offer_pda: (Pubkey, u8),
        program_pubkey: Pubkey,
        client: ArchRpcClient,
    ) {
        let test_config = Config::localnet();
        let bitcoin_network = test_config.network;

        mint_to(
            100,
            mint_b,
            taker_ata_b,
            maker_pubkey,
            maker_keypair,
            client.clone(),
        );

        let serialized_take_offer_input = borsh::to_vec(&EscrowInstruction::TakeOffer).unwrap();

        let take_offer_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[arch_program::instruction::Instruction {
                    program_id: program_pubkey,
                    accounts: vec![
                        AccountMeta::new(expected_offer_pda.0, false),
                        AccountMeta::new(mint_a, false),
                        AccountMeta::new(mint_b, false),
                        AccountMeta::new(maker_ata_b, false),
                        AccountMeta::new(taker_ata_a, false),
                        AccountMeta::new(taker_ata_b, false),
                        AccountMeta::new(vault, false),
                        AccountMeta::new(maker_pubkey, false),
                        AccountMeta::new(taker_pubkey, true),
                        AccountMeta::new_readonly(apl_token::id(), false),
                        AccountMeta::new_readonly(Pubkey::system_program(), false),
                        AccountMeta::new_readonly(apl_associated_token_account::id(), false),
                    ],
                    data: serialized_take_offer_input,
                }],
                Some(taker_pubkey),
                client.get_best_finalized_block_hash().unwrap(),
            ),
            vec![taker_keypair],
            bitcoin_network,
        )
        .expect("Failed to build and sign transaction");

        let txid = client.send_transaction(take_offer_tx).unwrap();
        let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();

        assert!(processed_tx.status == Status::Processed);
    }

    pub fn mint_to(
        mint_amount: u64,
        mint_pubkey: Pubkey,
        user_ata: Pubkey,
        user_pubkey: Pubkey,
        user_keypair: Keypair,
        client: ArchRpcClient,
    ) {
        let test_config = Config::localnet();
        let bitcoin_network = test_config.network;

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
                client.get_best_finalized_block_hash().unwrap(),
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
}
