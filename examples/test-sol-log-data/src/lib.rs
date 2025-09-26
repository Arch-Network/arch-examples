#![cfg(test)]
use arch_program::sanitized::ArchMessage;

use arch_program::instruction::Instruction;
use arch_sdk::{
    build_and_sign_transaction, with_secret_key_file, ArchRpcClient, Config, ProgramDeployer,
    Status,
};

pub const ELF_PATH: &str =
    "./program/target/sbpf-solana-solana/release/test_sol_log_data_program.so";

pub const PROGRAM_FILE_PATH: &str = ".sol_log_data_program.json";
pub const AUTHORITY_FILE_PATH: &str = ".sol_log_data_authority.json";

#[ignore]
#[test]
fn test_sol_log_data() {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let (program_keypair, _) =
        with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

    let (authority_keypair, authority_pubkey) =
        with_secret_key_file(AUTHORITY_FILE_PATH).expect("getting caller info should not fail");
    client
        .create_and_fund_account_with_faucet(&authority_keypair)
        .unwrap();

    let deployer = ProgramDeployer::new(&config);

    let program_pubkey = deployer
        .try_deploy_program(
            "Test Sol Log Data Program".to_string(),
            program_keypair,
            authority_keypair,
            &ELF_PATH.to_string(),
        )
        .unwrap();

    println!("PROGRAM CALL");

    let ins = Instruction {
        program_id: program_pubkey,
        accounts: vec![],
        data: vec![],
    };

    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[ins],
            Some(authority_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![authority_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(transaction).unwrap();

    let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();

    assert!(matches!(processed_tx.status, Status::Processed));

    let expected_log = "Program log: Testing sol_log_data syscall";
    assert!(processed_tx.logs.iter().any(|log| log == expected_log));

    let expected_log_2 = "Program data: SGVsbG8sIFdvcmxkISBUaGlzIGlzIGJpbmFyeSBkYXRhLg==";
    assert!(processed_tx
        .logs
        .iter()
        .any(|log| log.contains(expected_log_2)));

    println!("\x1b[32m Test successful :\x1b[0m sol_log_data test passed!",);

    for log in processed_tx.logs {
        println!("{}", log);
    }
}
