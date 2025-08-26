#![cfg(test)]
use arch_program::sanitized::ArchMessage;
use bitcoin::XOnlyPublicKey;

use arch_program::account::MIN_ACCOUNT_LAMPORTS;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction;
use arch_sdk::{build_and_sign_transaction, generate_new_keypair, ArchRpcClient, Config};

#[ignore]
#[test]
#[should_panic]
fn poc_duplicate_signed_txs() {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config.arch_node_url);

    let (authority_keypair, authority_pubkey, _) = generate_new_keypair(config.network);
    client
        .create_and_fund_account_with_faucet(&authority_keypair, config.network)
        .unwrap();

    let fee_payer_pubkey = Pubkey::from_slice(
        &XOnlyPublicKey::from_keypair(&authority_keypair)
            .0
            .serialize(),
    );
    let (account_keypair, account_pubkey, _) = generate_new_keypair(config.network);
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
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![authority_keypair, account_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txids = client.send_transactions(vec![tx.clone(), tx]).unwrap();
    let _processed_tx = client.wait_for_processed_transactions(txids);
}
