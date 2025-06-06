#[cfg(test)]
mod tests {
    use apl_token::state::Mint;
    use arch_program::{program_pack::Pack, sanitized::ArchMessage};
    use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Status};
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, NODE1_ADDRESS},
        helper::{
            create_and_fund_account_with_faucet, read_account_info, send_transactions_and_wait,
        },
        instructions::*,
        logging::init_logging,
    };
    use serial_test::serial;

    #[ignore]
    #[test]
    #[serial]
    fn test_initialize_mint() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (token_mint_keypair, token_mint_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        create_account_helper(
            &client,
            &authority_pubkey,
            &token_mint_pubkey,
            authority_keypair,
            token_mint_keypair,
            Mint::LEN as u64,
        );

        let initialize_mint_instruction = apl_token::instruction::initialize_mint(
            &apl_token::id(),
            &token_mint_pubkey,
            &authority_pubkey,
            None,
            9,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[initialize_mint_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, token_mint_keypair],
            BITCOIN_NETWORK,
        );

        let processed_transactions = send_transactions_and_wait(vec![transaction]);
        assert!(processed_transactions[0].status == Status::Processed);
        dbg!(processed_transactions[0].compute_units_consumed()); // 1311

        let token_mint_info = read_account_info(token_mint_pubkey);
        let token_mint_data = Mint::unpack(&token_mint_info.data).unwrap();

        assert_eq!(token_mint_data.decimals, 9);
        assert!(token_mint_data.is_initialized);
        assert_eq!(token_mint_data.mint_authority.unwrap(), authority_pubkey);
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_initialize_account() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (token_account_keypair, token_account_pubkey, _) =
            generate_new_keypair(BITCOIN_NETWORK);

        let (_token_mint_keypair, token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

        create_account_helper(
            &client,
            &authority_pubkey,
            &token_account_pubkey,
            authority_keypair,
            token_account_keypair,
            apl_token::state::Account::LEN as u64,
        );

        let initialize_token_account_instruction = apl_token::instruction::initialize_account(
            &apl_token::id(),
            &token_account_pubkey,
            &token_mint_pubkey,
            &authority_pubkey,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[initialize_token_account_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_transactions = send_transactions_and_wait(vec![transaction]);
        dbg!(processed_transactions[0].compute_units_consumed()); // 1514
        assert!(processed_transactions[0].status == Status::Processed);

        let token_account_data = read_account_info(token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        assert_eq!(token_account.mint, token_mint_pubkey);
        assert_eq!(token_account.owner, authority_pubkey);
        assert_eq!(
            token_account.state,
            apl_token::state::AccountState::Initialized
        );
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_initializ_multisig() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);
        let (authority_keypair2, authority_pubkey2, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (multisig_keypair, multisig_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        create_account_helper(
            &client,
            &authority_pubkey,
            &multisig_pubkey,
            authority_keypair,
            multisig_keypair,
            apl_token::state::Multisig::LEN as u64,
        );

        let initialize_multisig_instruction = apl_token::instruction::initialize_multisig(
            &apl_token::id(),
            &multisig_pubkey,
            &[&authority_pubkey, &authority_pubkey2],
            2,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[initialize_multisig_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, authority_keypair2],
            BITCOIN_NETWORK,
        );

        let processed_transactions = send_transactions_and_wait(vec![transaction]);
        dbg!(processed_transactions[0].compute_units_consumed()); // 1997
        assert!(processed_transactions[0].status == Status::Processed);

        let multisig_account_info = read_account_info(multisig_pubkey);
        let multisig_data: apl_token::state::Multisig =
            apl_token::state::Multisig::unpack(&multisig_account_info.data).unwrap();

        assert!(multisig_data.is_initialized);
        assert_eq!(multisig_data.signers[0], authority_pubkey);
        assert_eq!(multisig_data.signers[1], authority_pubkey2);
        assert_eq!(multisig_data.n, 2);
        assert_eq!(multisig_data.m, 2);
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_transfer() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (recipient_keypair, recipient_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&recipient_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

        // // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        // create token account for `recipient_keypair`
        let (_, recipient_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, recipient_keypair);

        mint_tokens(
            &client,
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        let transfer_instruction = apl_token::instruction::transfer(
            &apl_token::id(),
            &authority_token_account_pubkey,
            &recipient_token_account_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
            50,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[transfer_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_transactions = send_transactions_and_wait(vec![transaction]);
        dbg!(&processed_transactions[0].compute_units_consumed()); // 3128
        assert!(processed_transactions[0].status == Status::Processed);

        let recipient_token_account_info = read_account_info(recipient_token_account_pubkey);
        let data = apl_token::state::Account::unpack(&recipient_token_account_info.data).unwrap();
        assert_eq!(data.amount, 50);
        assert_eq!(data.state, apl_token::state::AccountState::Initialized);
        assert_eq!(data.mint, token_mint_pubkey);
        assert_eq!(data.owner, recipient_pubkey);
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_approve() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (recipient_keypair, _recipient_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&recipient_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

        // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        // create token account for `recipient_keypair`
        let (_, recipient_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, recipient_keypair);

        mint_tokens(
            &client,
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        let approve_instruction = apl_token::instruction::approve(
            &apl_token::id(),
            &authority_token_account_pubkey,
            &recipient_token_account_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
            50,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[approve_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_transactions = send_transactions_and_wait(vec![transaction]);
        dbg!(processed_transactions[0].compute_units_consumed()); // 1988
        assert!(processed_transactions[0].status == Status::Processed);

        let authority_token_account_info = read_account_info(authority_token_account_pubkey);
        let authority_token_account_data =
            apl_token::state::Account::unpack(&authority_token_account_info.data).unwrap();

        assert_eq!(authority_token_account_data.amount, 100);
        assert_eq!(authority_token_account_data.delegated_amount, 50);
        assert_eq!(
            authority_token_account_data.delegate.unwrap(),
            recipient_token_account_pubkey
        );
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_revoke() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (recipient_keypair, _recipient_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&recipient_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

        // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        // create token account for `recipient_keypair`
        let (_, recipient_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, recipient_keypair);

        mint_tokens(
            &client,
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        approve(
            &client,
            &authority_token_account_pubkey,
            &recipient_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            50,
        );

        let authority_token_account_info = read_account_info(authority_token_account_pubkey);
        let authority_token_account_data =
            apl_token::state::Account::unpack(&authority_token_account_info.data).unwrap();

        assert_eq!(authority_token_account_data.delegated_amount, 50);
        assert_eq!(
            authority_token_account_data.delegate.unwrap(),
            recipient_token_account_pubkey
        );

        let revoke_instruction = apl_token::instruction::revoke(
            &apl_token::id(),
            &authority_token_account_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[revoke_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![transaction]);
        dbg!(&processed_tx[0].compute_units_consumed()); // 1935
        assert_eq!(processed_tx[0].status, Status::Processed);

        let authority_token_account_info = read_account_info(authority_token_account_pubkey);
        let authority_token_account_data =
            apl_token::state::Account::unpack(&authority_token_account_info.data).unwrap();

        assert_eq!(authority_token_account_data.delegated_amount, 0);
    }

    // #[serial]
    // #[test]
    // fn test_set_authority() {
    //     init_logging();

    //     let client = ArchRpcClient::new(NODE1_ADDRESS);

    //     let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    //     create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    //     let (new_authority_keypair, new_authority_pubkey, _) =
    //         generate_new_keypair(BITCOIN_NETWORK);
    //     create_and_fund_account_with_faucet(&new_authority_keypair, BITCOIN_NETWORK);

    //     // create mint
    //     let (_, token_mint_pubkey) =
    //         initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

    //     let set_authority_instruction = apl_token::instruction::set_authority(
    //         &apl_token::id(),
    //         &token_mint_pubkey,
    //         Some(&new_authority_pubkey),
    //         // AuthorityType::AccountOwner,
    //         // AuthorityType::FreezeAccount,
    //         apl_token::instruction::AuthorityType::CloseAccount,
    //         &authority_pubkey,
    //         &[&authority_pubkey],
    //     )
    //     .unwrap();

    //     let transaction = build_and_sign_transaction(
    //         ArchMessage::new(
    //             &[set_authority_instruction],
    //             Some(authority_pubkey),
    //             client.get_best_block_hash().unwrap(),
    //         ),
    //         vec![authority_keypair],
    //         BITCOIN_NETWORK,
    //     );

    //     let processed_tx = send_transactions_and_wait(vec![transaction]);
    //     assert_eq!(processed_tx[0].status, Status::Processed);

    //     let token_mint_info = read_account_info(token_mint_pubkey);
    //     let token_mint_data = Mint::unpack(&token_mint_info.data).unwrap();

    //     assert_eq!(
    //         token_mint_data.mint_authority.unwrap(),
    //         new_authority_pubkey
    //     );
    // }

    #[ignore]
    #[serial]
    #[test]
    fn test_mint_to() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

        // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        let mint_to_instruction = apl_token::instruction::mint_to(
            &apl_token::id(),
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
            100,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[mint_to_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![transaction]);
        dbg!(&processed_tx[0].compute_units_consumed()); // 3355
        assert_eq!(processed_tx[0].status, Status::Processed);

        let authority_token_account_info = read_account_info(authority_token_account_pubkey);
        let authority_token_account_data =
            apl_token::state::Account::unpack(&authority_token_account_info.data).unwrap();

        assert_eq!(authority_token_account_data.amount, 100);
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_burn() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

        // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        mint_tokens(
            &client,
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        let authority_token_account_info = read_account_info(authority_token_account_pubkey);
        let authority_token_account_data =
            apl_token::state::Account::unpack(&authority_token_account_info.data).unwrap();

        assert_eq!(authority_token_account_data.amount, 100);

        let burn_instruction = apl_token::instruction::burn(
            &apl_token::id(),
            &authority_token_account_pubkey,
            &token_mint_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
            100,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[burn_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![transaction]);
        dbg!(&processed_tx[0].compute_units_consumed()); // 3101
        assert_eq!(processed_tx[0].status, Status::Processed);

        let authority_token_account_info = read_account_info(authority_token_account_pubkey);
        let authority_token_account_data =
            apl_token::state::Account::unpack(&authority_token_account_info.data).unwrap();

        assert_eq!(authority_token_account_data.amount, 0);
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_close_account() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (destination_keypair, _destination_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&destination_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair, None);

        // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        // mint tokens for `authority_token_account_pubkey`
        mint_tokens(
            &client,
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        // create token account for `destination_keypair`
        let (_, destination_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, destination_keypair);

        let close_account_instruction = apl_token::instruction::close_account(
            &apl_token::id(),
            &authority_token_account_pubkey,
            &destination_token_account_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
        )
        .unwrap();

        let token_account_data = read_account_info(authority_token_account_pubkey).data;
        assert!(!token_account_data.is_empty());
        assert!(read_account_info(authority_token_account_pubkey).lamports > 0);

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[close_account_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![transaction]);
        dbg!(&processed_tx[0].compute_units_consumed()); // 2815
        assert_eq!(processed_tx[0].status, Status::Processed);

        let token_account_data = read_account_info(authority_token_account_pubkey).data;
        assert!(token_account_data.is_empty());
        assert!(read_account_info(authority_token_account_pubkey).lamports == 0);
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_freeze_account() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) = initialize_mint_token(
            &client,
            authority_pubkey,
            authority_keypair,
            Some(&authority_pubkey), // set authority_pubkey as freeze authoriy
        );

        // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        mint_tokens(
            &client,
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        // Freeze the account
        let freeze_instruction = apl_token::instruction::freeze_account(
            &apl_token::id(),
            &authority_token_account_pubkey,
            &token_mint_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[freeze_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![transaction]);
        dbg!(&processed_tx[0].compute_units_consumed()); // 3100
        assert_eq!(processed_tx[0].status, Status::Processed);

        // Verify account is frozen
        let token_account_info = read_account_info(authority_token_account_pubkey);
        let token_account_data =
            apl_token::state::Account::unpack(&token_account_info.data).unwrap();
        assert_eq!(
            token_account_data.state,
            apl_token::state::AccountState::Frozen
        );
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_thaw_account() {
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        // create mint
        let (_, token_mint_pubkey) = initialize_mint_token(
            &client,
            authority_pubkey,
            authority_keypair,
            Some(&authority_pubkey), // set authority_pubkey as freeze authoriy
        );

        // create token account for `authority_keypair`
        let (_, authority_token_account_pubkey) =
            initialize_token_account(&client, token_mint_pubkey, authority_keypair);

        mint_tokens(
            &client,
            &token_mint_pubkey,
            &authority_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        freeze_account(
            &client,
            &authority_token_account_pubkey,
            &token_mint_pubkey,
            &authority_pubkey,
            authority_keypair,
        );

        let data = read_account_info(authority_token_account_pubkey).data;
        let token_data = apl_token::state::Account::unpack(&data).unwrap();
        assert_eq!(token_data.state, apl_token::state::AccountState::Frozen);

        let thaw_instruction = apl_token::instruction::thaw_account(
            &apl_token::id(),
            &authority_token_account_pubkey,
            &token_mint_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[thaw_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        );

        let processed_tx = send_transactions_and_wait(vec![transaction]);
        dbg!(processed_tx[0].compute_units_consumed()); // 3095
        assert_eq!(processed_tx[0].status, Status::Processed);

        let data = read_account_info(authority_token_account_pubkey).data;
        let token_data = apl_token::state::Account::unpack(&data).unwrap();
        assert_eq!(
            token_data.state,
            apl_token::state::AccountState::Initialized
        );
    }

    // #[test
    // #[serial]
    // fn test_transfer_multisig() {}
}
