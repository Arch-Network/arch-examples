#![cfg(test)]
use arch_program::sanitized::ArchMessage;
use bitcoin::XOnlyPublicKey;

use arch_program::account::MIN_ACCOUNT_LAMPORTS;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction;
use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient};
use arch_test_sdk::{
    constants::{BITCOIN_NETWORK, NODE1_ADDRESS},
    helper::{create_and_fund_account_with_faucet, send_transactions_and_wait},
};

#[ignore]
#[test]
#[should_panic]
fn poc_duplicate_signed_txs() {
    let client = ArchRpcClient::new(NODE1_ADDRESS);

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

    let fee_payer_pubkey = Pubkey::from_slice(
        &XOnlyPublicKey::from_keypair(&authority_keypair)
            .0
            .serialize(),
    );
    let (account_keypair, account_pubkey, _) = generate_new_keypair(BITCOIN_NETWORK);
    println!("Account created with address, {:?}", account_pubkey.0);

    let tx = build_and_sign_transaction(
        ArchMessage::new(
            &[system_instruction::create_account(
                &fee_payer_pubkey,
                &account_pubkey,
                MIN_ACCOUNT_LAMPORTS,
                0,
                &authority_pubkey,
            )],
            Some(fee_payer_pubkey),
            client.get_best_block_hash().unwrap(),
        ),
        vec![authority_keypair, account_keypair],
        BITCOIN_NETWORK,
    )
    .expect("Failed to build and sign transaction");

    let _processed_tx = send_transactions_and_wait(vec![tx.clone(), tx]);
}
