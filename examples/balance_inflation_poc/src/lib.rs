#![cfg(test)]
use arch_program::sanitized::ArchMessage;
use arch_test_sdk::constants::CALLER_FILE_PATH;
use bitcoin::XOnlyPublicKey;

use arch_program::account::{AccountMeta, MIN_ACCOUNT_LAMPORTS};
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Config,
    Status,
};
use arch_test_sdk::{
    constants::{BITCOIN_NETWORK, PROGRAM_FILE_PATH},
    helper::{deploy_program, read_account_info, send_transactions_and_wait, send_utxo},
};

pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/balance_inflation_poc.so";

#[ignore]
#[test]
fn poc_inflate_balance() {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let program_pubkey = deploy_program(
        "Balance Inflation POC".to_string(),
        ELF_PATH.to_string(),
        program_keypair,
        authority_keypair,
    );

    let fee_payer_pubkey = Pubkey::from_slice(
        &XOnlyPublicKey::from_keypair(&authority_keypair)
            .0
            .serialize(),
    );
    let (account_keypair, account_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    let (txid, vout) = send_utxo(account_pubkey);
    println!("Account created with address, {:?}", account_pubkey.0);

    let tx = build_and_sign_transaction(
        ArchMessage::new(
            &[system_instruction::create_account_with_anchor(
                &fee_payer_pubkey,
                &account_pubkey,
                MIN_ACCOUNT_LAMPORTS,
                0,
                &program_pubkey,
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
            )],
            Some(fee_payer_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![authority_keypair, account_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let processed_tx = send_transactions_and_wait(vec![tx]);
    assert_eq!(
        processed_tx[0].status,
        Status::Processed,
        "Account creation transaction failed"
    );

    let instruction = Instruction {
        program_id: program_pubkey,
        accounts: vec![
            AccountMeta {
                pubkey: account_pubkey,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: fee_payer_pubkey,
                is_signer: true,
                is_writable: true,
            },
            // Repeat this account. It will get adjusted below.
            AccountMeta {
                pubkey: account_pubkey,
                is_signer: true,
                is_writable: true,
            },
        ],
        data: vec![],
    };

    let mut message = ArchMessage::new(
        &[instruction],
        Some(authority_pubkey),
        client.get_best_finalized_block_hash().unwrap(),
    );

    // Add repeated entry of the same pubkey
    message.account_keys.push(account_pubkey);
    // Adjust the first account in the instruction to point to the second instance of the pubkey
    message.instructions[0].accounts[0] = 3;
    // Set all accounts as writable, it's more convenient to do it this way
    message.header.num_readonly_unsigned_accounts = 0;

    dbg!("MESSAGE:", &message);

    let transaction = build_and_sign_transaction(
        message,
        vec![account_keypair, authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let account_balance_before = read_account_info(account_pubkey).lamports;
    dbg!("Account balance before: ", account_balance_before);

    let block_transactions = send_transactions_and_wait(vec![transaction]);

    assert_eq!(
        block_transactions[0].status,
        Status::Failed(
            "verify_and_prepare_block: failed to process transaction: duplicate account keys"
                .to_string()
        ),
        "Transaction shouldn't have passed"
    );

    let account_balance_after = read_account_info(account_pubkey).lamports;
    dbg!("Account balance after: ", account_balance_after);

    assert_ne!(
        account_balance_after,
        account_balance_before * 2,
        "Account balance after the TX shouldn't be doubled"
    );
}

#[ignore]
#[test]
fn testing_pay_fees_dos() {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (_program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey) =
        with_secret_key_file(CALLER_FILE_PATH).expect("getting caller info should not fail");
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let (first_account_keypair, first_account_pubkey, _address) =
        generate_new_keypair(config.network);

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[system_instruction::transfer(
                &authority_pubkey,
                &first_account_pubkey,
                256,
            )],
            Some(authority_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![authority_keypair],
        config.network,
    )
    .unwrap();

    let _block_transactions = send_transactions_and_wait(vec![transaction.clone()]);

    let authority_balance_before = read_account_info(authority_pubkey).lamports;
    dbg!("Authority balance before: ", authority_balance_before);

    let account_balance_before = read_account_info(first_account_pubkey).lamports;
    dbg!("Account balance before: ", account_balance_before);

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[system_instruction::transfer(
                &first_account_pubkey,
                &first_account_pubkey,
                0,
            )],
            Some(first_account_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![first_account_keypair],
        BITCOIN_NETWORK,
    )
    .unwrap();

    let block_transactions = send_transactions_and_wait(vec![transaction.clone()]);

    // It should fail because the account doesn't have enough lamports to pay the fees
    // we cancel the transaction in this case
    assert!(matches!(block_transactions[0].status, Status::Failed(_)));

    let account_balance_after = read_account_info(first_account_pubkey).lamports;
    dbg!("Account balance after: ", account_balance_after);
}
