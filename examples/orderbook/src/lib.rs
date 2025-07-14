#[cfg(test)]
mod tests {
    use apl_token::state::Mint;
    use arch_program::{
        account::{AccountMeta, MIN_ACCOUNT_LAMPORTS},
        instruction::Instruction,
        program_pack::Pack,
        pubkey::Pubkey,
        sanitized::ArchMessage,
        system_instruction::create_account,
        system_program::SYSTEM_PROGRAM_ID,
    };
    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
        Status,
    };
    use arch_test_sdk::{
        constants::{BITCOIN_NETWORK, NODE1_ADDRESS},
        helper::{
            create_and_fund_account_with_faucet, deploy_program, read_account_info,
            send_transactions_and_wait, send_utxo,
        },
        logging::init_logging,
    };
    use bitcoin::key::Keypair;
    use serial_test::serial;
    use std::sync::{Arc, Condvar, Mutex};

    use orderbook_program::{
        instruction::OrderbookInstruction,
        state::{Order, OrderbookState, Side},
    };

    pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/orderbook_program.so";

    fn initialize_orderbook(
        client: &ArchRpcClient,
        program_pubkey: Pubkey,
        first_token_mint_pubkey: Pubkey,
        second_token_mint_pubkey: Pubkey,
        authority_pubkey: Pubkey,
        authority_keypair: Keypair,
    ) -> Pubkey {
        // Create orderbook account
        let (orderbook_pubkey, _) = Pubkey::try_find_program_address(
            &[
                b"orderbook",
                first_token_mint_pubkey.as_ref(),
                second_token_mint_pubkey.as_ref(),
            ],
            &program_pubkey,
        )
        .unwrap();

        let init_instruction = Instruction {
            program_id: program_pubkey,
            accounts: vec![
                AccountMeta::new(orderbook_pubkey, false),
                AccountMeta::new_readonly(first_token_mint_pubkey, false),
                AccountMeta::new_readonly(second_token_mint_pubkey, false),
                AccountMeta::new(authority_pubkey, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            ],
            data: borsh::to_vec(&OrderbookInstruction::InitializeOrderbook).unwrap(),
        };

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[init_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }

        assert!(!matches!(
            processed_transactions[0].status,
            Status::Failed { .. }
        ));

        // Verify orderbook state
        let account_info = read_account_info(orderbook_pubkey);
        let data = account_info.data;
        let orderbook_state: OrderbookState = unsafe { *(data.as_ptr() as *mut OrderbookState) };

        assert!(orderbook_state.initialized);
        assert_eq!(orderbook_state.first_token_mint, first_token_mint_pubkey);
        assert_eq!(orderbook_state.second_token_mint, second_token_mint_pubkey);

        orderbook_pubkey
    }

    // #[test]
    fn mint_tokens_test() {
        let max_outstanding = 100;
        let outstanding = Arc::new((Mutex::new(0_u64), Condvar::new()));
        for i in 0..1000000 {
            loop {
                let (lock, cv) = &*outstanding;
                let mut current = lock.lock().unwrap();
                if *current < max_outstanding {
                    *current += 1;
                    println!(
                        "mint_tokens_test: current = {current}, minting tokens: {}",
                        i
                    );
                    break;
                }
                println!("mint_tokens_test: current = {current}, to wait ...");
                std::mem::drop(cv.wait(current).unwrap());
            }

            let outstanding_cl = outstanding.clone();
            std::thread::spawn(move || {
                init_logging();
                let client = ArchRpcClient::new(NODE1_ADDRESS);

                let (authority_keypair, authority_pubkey, _) =
                    generate_new_keypair(BITCOIN_NETWORK);
                create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

                // Initialize mint token
                let (_, first_token_mint_pubkey) =
                    initialize_mint_token(&client, authority_pubkey, authority_keypair);

                // Create token account
                let (_, first_token_account_pubkey) =
                    initialize_token_account(&client, first_token_mint_pubkey, authority_keypair);

                // Mint tokens
                mint_tokens(
                    &client,
                    &first_token_mint_pubkey,
                    &first_token_account_pubkey,
                    &authority_pubkey,
                    authority_keypair,
                    100,
                );

                let (lock, cv) = &*outstanding_cl;
                let mut current = lock.lock().unwrap();
                *current -= 1;
                cv.notify_one();
            });
        }
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_initialize_orderbook() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) = with_secret_key_file("./program.json").unwrap();

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let (_, first_token_mint_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        let (_, second_token_mint_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "Orderbook Program".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );

        initialize_orderbook(
            &client,
            program_pubkey,
            first_token_mint_pubkey,
            second_token_mint_pubkey,
            authority_pubkey,
            authority_keypair,
        );
    }

    fn initialize_mint_token(
        client: &ArchRpcClient,
        authority_pubkey: Pubkey,
        authority_keypair: Keypair,
    ) -> (Keypair, Pubkey) {
        init_logging();

        let (token_mint_keypair, token_mint_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);

        let create_account_instruction = create_account(
            &authority_pubkey,
            &token_mint_pubkey,
            MIN_ACCOUNT_LAMPORTS,
            Mint::LEN as u64,
            &apl_token::id(),
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
                &[create_account_instruction, initialize_mint_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair, token_mint_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }

        (token_mint_keypair, token_mint_pubkey)
    }

    fn initialize_token_account(
        client: &ArchRpcClient,
        token_mint_pubkey: Pubkey,
        owner_keypair: Keypair,
    ) -> (Keypair, Pubkey) {
        init_logging();

        let owner_pubkey = Pubkey::from_slice(&owner_keypair.x_only_public_key().0.serialize());

        let (token_account_keypair, token_account_pubkey, _) =
            generate_new_keypair(BITCOIN_NETWORK);

        let create_account_instruction = create_account(
            &owner_pubkey,
            &token_account_pubkey,
            MIN_ACCOUNT_LAMPORTS,
            apl_token::state::Account::LEN as u64,
            &apl_token::id(),
        );

        let initialize_token_account_instruction = apl_token::instruction::initialize_account(
            &apl_token::id(),
            &token_account_pubkey,
            &token_mint_pubkey,
            &owner_pubkey,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[
                    create_account_instruction,
                    initialize_token_account_instruction,
                ],
                Some(owner_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![owner_keypair, token_account_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }

        let token_account_data = read_account_info(token_account_pubkey);
        println!("Token account data: {:?}", token_account_data);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Token account: {:?}", token_account);

        (token_account_keypair, token_account_pubkey)
    }

    fn mint_tokens(
        client: &ArchRpcClient,
        mint_pubkey: &Pubkey,
        account_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
        owner_keypair: Keypair,
        amount: u64,
    ) {
        let instruction = apl_token::instruction::mint_to(
            &apl_token::id(),
            mint_pubkey,
            account_pubkey,
            owner_pubkey,
            &[],
            amount,
        )
        .unwrap();

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(*owner_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![owner_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_place_order() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) = with_secret_key_file("./program.json").unwrap();

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "Orderbook Program".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );

        // Initialize mint token
        let (_, first_token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair);
        let (_, second_token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair);

        // Create token account
        let (_, first_token_account_pubkey) =
            initialize_token_account(&client, first_token_mint_pubkey, authority_keypair);
        let (_, second_token_account_pubkey) =
            initialize_token_account(&client, second_token_mint_pubkey, authority_keypair);

        // Mint tokens
        mint_tokens(
            &client,
            &first_token_mint_pubkey,
            &first_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        mint_tokens(
            &client,
            &second_token_mint_pubkey,
            &second_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        // Verify token accounts
        let first_token_account_data = read_account_info(first_token_account_pubkey);
        let first_token_account =
            apl_token::state::Account::unpack_from_slice(&first_token_account_data.data).unwrap();
        assert_eq!(first_token_account.amount, 100);

        let second_token_account_data = read_account_info(second_token_account_pubkey);
        let second_token_account =
            apl_token::state::Account::unpack_from_slice(&second_token_account_data.data).unwrap();
        assert_eq!(second_token_account.amount, 100);

        // Initialize orderbook first
        let orderbook_pubkey = initialize_orderbook(
            &client,
            program_pubkey,
            first_token_mint_pubkey,
            second_token_mint_pubkey,
            authority_pubkey,
            authority_keypair,
        );

        // Place multiple ask orders
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            100,
            8,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            98,
            12,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            102,
            3,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            95,
            7,
        );

        // Place multiple bid orders
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            100,
            10,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            98,
            5,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            102,
            15,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            95,
            20,
        );

        print_orders(&orderbook_pubkey);

        // let orderbook_info = read_account_info(orderbook_pubkey);
        // let orderbook_state: OrderbookState =
        //     unsafe { *(orderbook_info.data.as_ptr() as *const OrderbookState) };
        // assert_eq!(orderbook_state.num_orders, 2);

        // let order1 =
        //     Order::unpack_from_slice(&orderbook_info.data[std::mem::size_of::<OrderbookState>()..])
        //         .unwrap();
        // println!("order1: {:?}", order1);
        // assert_eq!(order1.side, Side::Bid);
        // assert_eq!(order1.price, 100);
        // assert_eq!(order1.size, 10);

        // let order2 = Order::unpack_from_slice(
        //     &orderbook_info.data
        //         [std::mem::size_of::<OrderbookState>() + std::mem::size_of::<Order>()..],
        // )
        // .unwrap();
        // println!("order2: {:?}", order2);
        // assert_eq!(order2.side, Side::Ask);
        // assert_eq!(order2.price, 99);
        // assert_eq!(order2.size, 10);
    }

    fn print_orders(orderbook_pubkey: &Pubkey) {
        let orderbook_info = read_account_info(*orderbook_pubkey);
        let orderbook_state: OrderbookState =
            unsafe { *(orderbook_info.data.as_ptr() as *const OrderbookState) };

        // Print all orders to verify sorting
        let mut offset = std::mem::size_of::<OrderbookState>();
        for i in 0..orderbook_state.num_orders {
            let order = Order::unpack_from_slice(&orderbook_info.data[offset..]).unwrap();
            println!(
                "Order {}: side={:?}, price={}, size={}",
                i, order.side, order.price, order.size
            );
            offset += std::mem::size_of::<Order>();
        }
    }

    fn place_limit_order(
        client: &ArchRpcClient,
        program_pubkey: &Pubkey,
        orderbook_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        authority_keypair: &Keypair,
        first_token_mint_pubkey: &Pubkey,
        second_token_mint_pubkey: &Pubkey,
        first_token_account_pubkey: &Pubkey,
        second_token_account_pubkey: &Pubkey,
        side: Side,
        price: u64,
        size: u64,
    ) {
        // Place order instruction
        let place_order_instruction = Instruction {
            program_id: *program_pubkey,
            accounts: vec![
                AccountMeta::new(*orderbook_pubkey, false),
                AccountMeta::new(*first_token_mint_pubkey, false),
                AccountMeta::new(*second_token_mint_pubkey, false),
                AccountMeta::new(*authority_pubkey, true),
                AccountMeta::new(*first_token_account_pubkey, false),
                AccountMeta::new(*second_token_account_pubkey, false),
                AccountMeta::new_readonly(apl_token::id(), false),
            ],
            data: borsh::to_vec(&OrderbookInstruction::PlaceLimitOrder { side, price, size })
                .unwrap(),
        };

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[place_order_instruction],
                Some(*authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![*authority_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }

        assert!(!matches!(
            processed_transactions[0].status,
            Status::Failed { .. }
        ));
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_cancel_order() {
        init_logging();

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (program_keypair, _) = with_secret_key_file("./program.json").unwrap();

        let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "Orderbook Program".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );

        // Initialize mint token
        let (_, first_token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair);
        let (_, second_token_mint_pubkey) =
            initialize_mint_token(&client, authority_pubkey, authority_keypair);

        // Create token account
        let (_, first_token_account_pubkey) =
            initialize_token_account(&client, first_token_mint_pubkey, authority_keypair);
        let (_, second_token_account_pubkey) =
            initialize_token_account(&client, second_token_mint_pubkey, authority_keypair);

        // Mint tokens
        mint_tokens(
            &client,
            &first_token_mint_pubkey,
            &first_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        mint_tokens(
            &client,
            &second_token_mint_pubkey,
            &second_token_account_pubkey,
            &authority_pubkey,
            authority_keypair,
            100,
        );

        // Verify token accounts
        let first_token_account_data = read_account_info(first_token_account_pubkey);
        let first_token_account =
            apl_token::state::Account::unpack_from_slice(&first_token_account_data.data).unwrap();
        assert_eq!(first_token_account.amount, 100);

        let second_token_account_data = read_account_info(second_token_account_pubkey);
        let second_token_account =
            apl_token::state::Account::unpack_from_slice(&second_token_account_data.data).unwrap();
        assert_eq!(second_token_account.amount, 100);

        // Initialize orderbook first
        let orderbook_pubkey = initialize_orderbook(
            &client,
            program_pubkey,
            first_token_mint_pubkey,
            second_token_mint_pubkey,
            authority_pubkey,
            authority_keypair,
        );

        // Place multiple ask orders
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            100,
            8,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            98,
            12,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            102,
            3,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Ask,
            95,
            7,
        );

        // Place multiple bid orders
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            100,
            10,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            98,
            5,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            102,
            15,
        );
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            Side::Bid,
            95,
            20,
        );

        print_orders(&orderbook_pubkey);

        println!("Cancelling order 0");

        cancel_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &authority_pubkey,
            &authority_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &first_token_account_pubkey,
            &second_token_account_pubkey,
            0,
        );

        print_orders(&orderbook_pubkey);
    }

    fn cancel_order(
        client: &ArchRpcClient,
        program_pubkey: &Pubkey,
        orderbook_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
        owner_keypair: &Keypair,
        first_token_mint_pubkey: &Pubkey,
        second_token_mint_pubkey: &Pubkey,
        first_token_account_pubkey: &Pubkey,
        second_token_account_pubkey: &Pubkey,
        order_index: u32,
    ) {
        let cancel_order_instruction = Instruction {
            program_id: *program_pubkey,
            accounts: vec![
                AccountMeta::new(*orderbook_pubkey, false),
                AccountMeta::new(*first_token_mint_pubkey, false),
                AccountMeta::new(*second_token_mint_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
                AccountMeta::new(*first_token_account_pubkey, false),
                AccountMeta::new(*second_token_account_pubkey, false),
                AccountMeta::new_readonly(apl_token::id(), false),
            ],
            data: borsh::to_vec(&OrderbookInstruction::CancelOrder { order_index }).unwrap(),
        };

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[cancel_order_instruction],
                Some(*owner_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![*owner_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }

        assert!(!matches!(
            processed_transactions[0].status,
            Status::Failed { .. }
        ));
    }

    #[ignore]
    #[test]
    #[serial]
    fn test_match_orders() {
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (owner_keypair, owner_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&owner_keypair, BITCOIN_NETWORK);

        // Initialize mint token
        let (_, first_token_mint_pubkey) =
            initialize_mint_token(&client, owner_pubkey, owner_keypair);
        let (_, second_token_mint_pubkey) =
            initialize_mint_token(&client, owner_pubkey, owner_keypair);

        // Create bid side token account
        let (_, bid_first_token_account_pubkey) =
            initialize_token_account(&client, first_token_mint_pubkey, owner_keypair);
        let (_, bid_second_token_account_pubkey) =
            initialize_token_account(&client, second_token_mint_pubkey, owner_keypair);

        // Create ask side token account
        let (_, ask_first_token_account_pubkey) =
            initialize_token_account(&client, first_token_mint_pubkey, owner_keypair);
        let (_, ask_second_token_account_pubkey) =
            initialize_token_account(&client, second_token_mint_pubkey, owner_keypair);

        mint_tokens(
            &client,
            &second_token_mint_pubkey,
            &bid_second_token_account_pubkey,
            &owner_pubkey,
            owner_keypair,
            100,
        );

        mint_tokens(
            &client,
            &first_token_mint_pubkey,
            &ask_first_token_account_pubkey,
            &owner_pubkey,
            owner_keypair,
            100,
        );

        let token_account_data = read_account_info(bid_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side first token account: {:?}", token_account);

        assert_eq!(token_account.amount, 0);

        let token_account_data = read_account_info(bid_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side second token account: {:?}", token_account);

        assert_eq!(token_account.amount, 100);

        let token_account_data = read_account_info(ask_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side first token account: {:?}", token_account);

        assert_eq!(token_account.amount, 100);

        let token_account_data = read_account_info(ask_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side second token account: {:?}", token_account);

        assert_eq!(token_account.amount, 0);

        let (program_keypair, _) = with_secret_key_file("./program.json").unwrap();
        let program_pubkey = deploy_program(
            "Orderbook Program".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            owner_keypair,
        );

        // Initialize orderbook
        let orderbook_pubkey = initialize_orderbook(
            &client,
            program_pubkey,
            first_token_mint_pubkey,
            second_token_mint_pubkey,
            owner_pubkey,
            owner_keypair,
        );

        // Place bid order
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &owner_pubkey,
            &owner_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &bid_first_token_account_pubkey,
            &bid_second_token_account_pubkey,
            Side::Bid,
            1,
            10,
        );

        // Place ask order
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &owner_pubkey,
            &owner_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &ask_first_token_account_pubkey,
            &ask_second_token_account_pubkey,
            Side::Ask,
            1,
            10,
        );

        print_orders(&orderbook_pubkey);

        // Match orders
        let match_orders_instruction = Instruction {
            program_id: program_pubkey,
            accounts: vec![
                AccountMeta::new(orderbook_pubkey, false),
                AccountMeta::new(first_token_mint_pubkey, false),
                AccountMeta::new(second_token_mint_pubkey, false),
                AccountMeta::new(bid_first_token_account_pubkey, false),
                AccountMeta::new(bid_second_token_account_pubkey, false),
                AccountMeta::new(ask_first_token_account_pubkey, false),
                AccountMeta::new(ask_second_token_account_pubkey, false),
                AccountMeta::new_readonly(apl_token::id(), false),
            ],
            data: borsh::to_vec(&OrderbookInstruction::MatchOrders).unwrap(),
        };

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[match_orders_instruction],
                Some(owner_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![owner_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }

        print_orders(&orderbook_pubkey);

        let token_account_data = read_account_info(bid_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side first token account: {:?}", token_account);

        assert_eq!(token_account.amount, 10);

        let token_account_data = read_account_info(bid_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side second token account: {:?}", token_account);

        assert_eq!(token_account.amount, 90);

        let token_account_data = read_account_info(ask_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side first token account: {:?}", token_account);

        assert_eq!(token_account.amount, 90);

        let token_account_data = read_account_info(ask_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side second token account: {:?}", token_account);

        assert_eq!(token_account.amount, 10);
    }

    fn create_associated_token_account(
        client: &ArchRpcClient,
        owner_pubkey: Pubkey,
        spl_token_mint_pubkey: Pubkey,
        funder_pubkey: Pubkey,
        funder_keypair: Keypair,
    ) {
        let (associated_token_account_pubkey, _) =
            apl_associated_token_account::get_associated_token_address_and_bump_seed(
                &owner_pubkey,
                &spl_token_mint_pubkey,
                &apl_associated_token_account::id(),
            );

        let (txid, vout) = send_utxo(associated_token_account_pubkey);
        let mut data = hex::decode(txid).unwrap();
        data.extend_from_slice(&vout.to_le_bytes());

        let instruction = Instruction {
            program_id: apl_associated_token_account::id(),
            accounts: vec![
                AccountMeta::new(funder_pubkey, true),
                AccountMeta::new(associated_token_account_pubkey, false),
                AccountMeta::new(owner_pubkey, false),
                AccountMeta::new_readonly(spl_token_mint_pubkey, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(apl_token::id(), false),
            ],
            data,
        };

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(funder_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![funder_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }
    }

    #[ignore]
    #[test]
    fn test_place_market_order() {
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (owner_keypair, owner_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&owner_keypair, BITCOIN_NETWORK);

        // Initialize mint token
        let (_, first_token_mint_pubkey) =
            initialize_mint_token(&client, owner_pubkey, owner_keypair);
        let (_, second_token_mint_pubkey) =
            initialize_mint_token(&client, owner_pubkey, owner_keypair);

        // Create bid side token account
        let (_, bid_first_token_account_pubkey) =
            initialize_token_account(&client, first_token_mint_pubkey, owner_keypair);
        let (_, bid_second_token_account_pubkey) =
            initialize_token_account(&client, second_token_mint_pubkey, owner_keypair);

        // Create ask side token account
        let (_, ask_first_token_account_pubkey) =
            initialize_token_account(&client, first_token_mint_pubkey, owner_keypair);
        let (_, ask_second_token_account_pubkey) =
            initialize_token_account(&client, second_token_mint_pubkey, owner_keypair);

        mint_tokens(
            &client,
            &second_token_mint_pubkey,
            &bid_second_token_account_pubkey,
            &owner_pubkey,
            owner_keypair,
            100,
        );

        mint_tokens(
            &client,
            &first_token_mint_pubkey,
            &ask_first_token_account_pubkey,
            &owner_pubkey,
            owner_keypair,
            100,
        );

        let token_account_data = read_account_info(bid_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side first token account: {:?}", token_account);

        assert_eq!(token_account.amount, 0);

        let token_account_data = read_account_info(bid_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side second token account: {:?}", token_account);

        assert_eq!(token_account.amount, 100);

        let token_account_data = read_account_info(ask_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side first token account: {:?}", token_account);

        assert_eq!(token_account.amount, 100);

        let token_account_data = read_account_info(ask_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side second token account: {:?}", token_account);

        assert_eq!(token_account.amount, 0);

        let (program_keypair, _) = with_secret_key_file("./program.json").unwrap();
        let program_pubkey = deploy_program(
            "Orderbook Program".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            owner_keypair,
        );

        // Initialize orderbook
        let orderbook_pubkey = initialize_orderbook(
            &client,
            program_pubkey,
            first_token_mint_pubkey,
            second_token_mint_pubkey,
            owner_pubkey,
            owner_keypair,
        );

        // Place bid order
        // place_limit_order(
        //     &program_pubkey,
        //     &orderbook_pubkey,
        //     &owner_pubkey,
        //     &owner_keypair,
        //     &first_token_mint_pubkey,
        //     &second_token_mint_pubkey,
        //     &bid_first_token_account_pubkey,
        //     &bid_second_token_account_pubkey,
        //     Side::Bid,
        //     1,
        //     10,
        // );

        // Place ask order
        place_limit_order(
            &client,
            &program_pubkey,
            &orderbook_pubkey,
            &owner_pubkey,
            &owner_keypair,
            &first_token_mint_pubkey,
            &second_token_mint_pubkey,
            &ask_first_token_account_pubkey,
            &ask_second_token_account_pubkey,
            Side::Ask,
            1,
            10,
        );

        print_orders(&orderbook_pubkey);

        // Match orders
        let match_orders_instruction = Instruction {
            program_id: program_pubkey,
            accounts: vec![
                AccountMeta::new(orderbook_pubkey, false),
                AccountMeta::new(first_token_mint_pubkey, false),
                AccountMeta::new(second_token_mint_pubkey, false),
                AccountMeta::new(owner_pubkey, false),
                AccountMeta::new(bid_first_token_account_pubkey, false),
                AccountMeta::new(bid_second_token_account_pubkey, false),
                AccountMeta::new(ask_first_token_account_pubkey, false),
                AccountMeta::new(ask_second_token_account_pubkey, false),
                AccountMeta::new_readonly(apl_token::id(), false),
            ],
            data: borsh::to_vec(&OrderbookInstruction::PlaceMarketOrder {
                side: Side::Bid,
                size: 10,
            })
            .unwrap(),
        };

        let transaction = build_and_sign_transaction(
            ArchMessage::new(
                &[match_orders_instruction],
                Some(owner_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![owner_keypair],
            BITCOIN_NETWORK,
        )
        .expect("Failed to build and sign transaction");

        let processed_transactions = send_transactions_and_wait(vec![transaction]);

        for log in processed_transactions[0].logs.iter() {
            println!("Log: {:?}", log);
        }

        print_orders(&orderbook_pubkey);

        let token_account_data = read_account_info(bid_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side first token account: {:?}", token_account);

        // assert_eq!(token_account.amount, 10);

        let token_account_data = read_account_info(bid_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Bid side second token account: {:?}", token_account);

        // assert_eq!(token_account.amount, 90);

        let token_account_data = read_account_info(ask_first_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side first token account: {:?}", token_account);

        // assert_eq!(token_account.amount, 90);

        let token_account_data = read_account_info(ask_second_token_account_pubkey);
        let token_account: apl_token::state::Account =
            apl_token::state::Account::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Ask side second token account: {:?}", token_account);

        // assert_eq!(token_account.amount, 10);
    }

    #[ignore]
    #[test]
    fn test_create_associated_token_account() {
        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let (owner_keypair, owner_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&owner_keypair, BITCOIN_NETWORK);

        let (funder_keypair, funder_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
        create_and_fund_account_with_faucet(&funder_keypair, BITCOIN_NETWORK);

        let (_, first_token_mint_pubkey) =
            initialize_mint_token(&client, owner_pubkey, owner_keypair);

        let token_account_data = read_account_info(first_token_mint_pubkey);
        let token_mint: apl_token::state::Mint =
            apl_token::state::Mint::unpack_from_slice(&token_account_data.data).unwrap();
        println!("Token mint: {:?}", token_mint);

        create_associated_token_account(
            &client,
            owner_pubkey,
            first_token_mint_pubkey,
            funder_pubkey,
            funder_keypair,
        );
    }
}
