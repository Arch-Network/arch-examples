#![cfg(test)]
use arch_program::sanitized::ArchMessage;
use bitcoin::key::UntweakedKeypair;
use bitcoin::XOnlyPublicKey;

use arch_program::account::{AccountMeta, MIN_ACCOUNT_LAMPORTS};
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient, Status,
};
use arch_test_sdk::{
    constants::{BITCOIN_NETWORK, NODE1_ADDRESS, PROGRAM_FILE_PATH},
    helper::{
        create_and_fund_account_with_faucet, deploy_program, read_account_info,
        send_transactions_and_wait, send_utxo,
    },
};

pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/same_lamports_poc.so";

#[ignore]
#[should_panic]
#[test]
fn poc_same_lamports() {
    let client = ArchRpcClient::new(NODE1_ADDRESS);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let program_pubkey = deploy_program(
        "same_lamports_poc".to_string(),
        ELF_PATH.to_string(),
        program_keypair,
        authority_keypair,
    );

    let fee_payer_pubkey = Pubkey::from_slice(
        &XOnlyPublicKey::from_keypair(&authority_keypair)
            .0
            .serialize(),
    );

    let (account_keypair1, account_pubkey1) = create_account(
        &client,
        authority_keypair,
        &program_pubkey,
        &fee_payer_pubkey,
    );
    let (account_keypair2, account_pubkey2) = create_account(
        &client,
        authority_keypair,
        &program_pubkey,
        &fee_payer_pubkey,
    );
    let (account_keypair3, account_pubkey3) = create_account(
        &client,
        authority_keypair,
        &program_pubkey,
        &fee_payer_pubkey,
    );

    let instruction1 = Instruction {
        program_id: program_pubkey,
        accounts: vec![
            AccountMeta {
                pubkey: account_pubkey1,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: fee_payer_pubkey,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: account_pubkey2,
                is_signer: true,
                is_writable: true,
            },
        ],
        data: vec![],
    };

    let mut instruction2 = instruction1.clone();
    instruction2.accounts[2].pubkey = account_pubkey3;

    let message1 = ArchMessage::new(
        &[instruction1],
        Some(authority_pubkey),
        client.get_best_block_hash().unwrap(),
    );

    let message2 = ArchMessage::new(
        &[instruction2],
        Some(authority_pubkey),
        client.get_best_block_hash().unwrap(),
    );

    dbg!("MESSAGE1:", &message1);
    dbg!("MESSAGE2:", &message2);

    let transaction1 = build_and_sign_transaction(
        message1,
        vec![account_keypair1, authority_keypair, account_keypair2],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let transaction2 = build_and_sign_transaction(
        message2,
        vec![account_keypair1, authority_keypair, account_keypair3],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let account1_balance_before = read_account_info(account_pubkey1).lamports;
    dbg!("Account 1 balance before: ", account1_balance_before);
    let account2_balance_before = read_account_info(account_pubkey2).lamports;
    dbg!("Account 2 balance before: ", account2_balance_before);

    let account3_balance_before = read_account_info(account_pubkey3).lamports;
    dbg!("Account 3 balance before: ", account3_balance_before);

    let block_transactions = send_transactions_and_wait(vec![transaction1, transaction2]);

    assert_eq!(
        block_transactions[0].status,
        Status::Processed,
        "Transaction failed processing"
    );

    assert_eq!(
        block_transactions[1].status,
        Status::Processed,
        "Transaction failed processing"
    );

    let account1_balance_after = read_account_info(account_pubkey1).lamports;
    dbg!("Account 1 balance after: ", account1_balance_after);

    let account2_balance_after = read_account_info(account_pubkey2).lamports;
    dbg!("Account 2 balance after: ", account2_balance_after);

    let account3_balance_after = read_account_info(account_pubkey3).lamports;
    dbg!("Account 3 balance after: ", account3_balance_after);

    assert_eq!(
        account1_balance_before * 2 + account2_balance_before + account3_balance_before,
        account1_balance_after + account2_balance_after + account3_balance_after,
        "account1_balance_after should be used twice"
    );
}

fn create_account(
    client: &ArchRpcClient,
    authority_keypair: UntweakedKeypair,
    program_pubkey: &Pubkey,
    fee_payer_pubkey: &Pubkey,
) -> (UntweakedKeypair, Pubkey) {
    let (account_keypair1, account_pubkey1, _) = generate_new_keypair(BITCOIN_NETWORK);
    let (txid1, vout1) = send_utxo(account_pubkey1);
    println!("Account 1 created with address, {:?}", account_pubkey1.0);

    let txid1 = build_and_sign_transaction(
        ArchMessage::new(
            &[system_instruction::create_account_with_anchor(
                fee_payer_pubkey,
                &account_pubkey1,
                MIN_ACCOUNT_LAMPORTS,
                0,
                &program_pubkey,
                hex::decode(txid1).unwrap().try_into().unwrap(),
                vout1,
            )],
            Some(*fee_payer_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![authority_keypair.clone(), account_keypair1],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let processed_tx = send_transactions_and_wait(vec![txid1]);
    assert_eq!(
        processed_tx[0].status,
        Status::Processed,
        "Account 1 creation transaction failed"
    );
    (account_keypair1, account_pubkey1)
}
