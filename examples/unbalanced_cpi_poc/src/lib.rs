#![cfg(test)]
use arch_program::sanitized::ArchMessage;
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

pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/unbalanced_cpi_poc.so";
#[ignore]
#[test]
fn poc_unbalanced_cpi() {
    let client = ArchRpcClient::new(NODE1_ADDRESS);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let program_pubkey = deploy_program(
        "Unbalanced CPI POC".to_string(),
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

    let txid = build_and_sign_transaction(
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
            client.get_best_block_hash().unwrap(),
        ),
        vec![authority_keypair.clone(), account_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let processed_tx = send_transactions_and_wait(vec![txid]);
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
            AccountMeta {
                pubkey: Pubkey::system_program(),
                is_signer: false,
                is_writable: false,
            },
        ],
        data: vec![],
    };

    let message = ArchMessage::new(
        &[instruction],
        Some(authority_pubkey),
        client.get_best_block_hash().unwrap(),
    );

    dbg!("MESSAGE:", &message);

    let transaction = build_and_sign_transaction(
        message,
        vec![account_keypair, authority_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let account_balance_before = read_account_info(account_pubkey).lamports;
    dbg!("Account balance before: ", account_balance_before);

    let block_transactions = send_transactions_and_wait(vec![transaction]);

    assert_eq!(
        block_transactions[0].status,
        Status::Failed("verify_and_prepare_block: failed to process transaction: transaction error Error processing Instruction 0, error: sum of account balances before and after instruction do not match".to_string()),
        "Transaction shouldn't have passed"
    );
    dbg!("Transaction status: ", block_transactions[0].status.clone());
    let account_balance_after = read_account_info(account_pubkey).lamports;
    dbg!("Account balance after: ", account_balance_after);

    assert_eq!(
        account_balance_after, account_balance_before,
        "Account balance should be same as transaction reverted"
    );
}
