#![cfg(test)]
pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/disable_cpi_program.so";
pub const CPI_ELF_PATH: &str =
    "./cpi_program/target/sbpf-solana-solana/release/cpi_disable_cpi_program.so";

#[cfg(test)]
mod tests {
    use arch_program::{account::AccountMeta, instruction::Instruction, sanitized::ArchMessage};
    use arch_sdk::{
        build_and_sign_transaction, generate_new_keypair, with_secret_key_file, ArchRpcClient,
        Status,
    };
    use arch_test_sdk::{
        constants::{
            BITCOIN_NETWORK, NODE1_ADDRESS, PROGRAM_AUTHORITY_FILE_PATH, PROGRAM_FILE_PATH,
        },
        helper::{
            create_and_fund_account_with_faucet, deploy_program, read_account_info,
            send_transactions_and_wait,
        },
        logging::{init_logging, log_scenario_end, log_scenario_start},
    };
    use serial_test::serial;

    use crate::{CPI_ELF_PATH, ELF_PATH};

    #[ignore]
    #[serial]
    #[test]
    fn test_direct_call() {
        init_logging();

        log_scenario_start(
            1,
            "Direct call",
            "Deploying program, and calling it directly (NO CPI) (should succeed)",
        );

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (authority_keypair, authority_pubkey) =
            with_secret_key_file(PROGRAM_AUTHORITY_FILE_PATH)
                .expect("getting caller info should not fail");
        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let program_pubkey = deploy_program(
            "Disable-CPI".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let instruction = Instruction::new(program_pubkey, vec![0], vec![]);
        let tx = build_and_sign_transaction(
            ArchMessage::new(
                &[instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair.clone()],
            BITCOIN_NETWORK,
        );

        let processed_transaction = send_transactions_and_wait(vec![tx.clone()])[0].clone();

        println!(
            "Processed transaction id : {}",
            processed_transaction.txid()
        );
        matches!(processed_transaction.status, Status::Processed);

        log_scenario_end(1, "Transaction processed successfully");
    }
    #[ignore]
    #[serial]
    #[test]
    fn test_cpi_call_enabled() {
        init_logging();

        log_scenario_start(
            2,
            "Enabled CPI",
            "Deploying program, and calling it with CPI (should succeed)",
        );

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (cpi_program_keypair, _, _) = generate_new_keypair(BITCOIN_NETWORK);

        let (authority_keypair, authority_pubkey) =
            with_secret_key_file(PROGRAM_AUTHORITY_FILE_PATH)
                .expect("getting caller info should not fail");

        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let base_program_pubkey = deploy_program(
            "Disable-CPI".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );

        let cpi_program_pubkey = deploy_program(
            "CPI-Disable-CPI".to_string(),
            CPI_ELF_PATH.to_string(),
            cpi_program_keypair,
            authority_keypair,
        );

        let cpi_instruction = Instruction::new(
            cpi_program_pubkey,
            // anything other than 1 enables CPI
            vec![0],
            vec![AccountMeta {
                pubkey: base_program_pubkey,
                is_signer: false,
                is_writable: false,
            }],
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let cpi_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[cpi_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair.clone()],
            BITCOIN_NETWORK,
        );

        let processed_transaction = send_transactions_and_wait(vec![cpi_tx.clone()])[0].clone();

        for log in processed_transaction.clone().logs {
            println!("Log: {}", log);
        }

        println!(
            "Processed transaction id : {}",
            processed_transaction.txid()
        );

        matches!(processed_transaction.status, Status::Processed);
    }

    #[ignore]
    #[serial]
    #[test]
    fn test_cpi_call_disabled() {
        init_logging();

        log_scenario_start(
            3,
            "Disabled CPI",
            "Deploying program, and calling it with CPI (should fail)",
        );

        let (program_keypair, _) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (cpi_program_keypair, _, _) = generate_new_keypair(BITCOIN_NETWORK);

        let (authority_keypair, authority_pubkey) =
            with_secret_key_file(PROGRAM_AUTHORITY_FILE_PATH)
                .expect("getting caller info should not fail");

        create_and_fund_account_with_faucet(&authority_keypair, BITCOIN_NETWORK);

        let base_program_pubkey = deploy_program(
            "Disable-CPI".to_string(),
            ELF_PATH.to_string(),
            program_keypair,
            authority_keypair,
        );

        let cpi_program_pubkey = deploy_program(
            "CPI-Disable-CPI".to_string(),
            CPI_ELF_PATH.to_string(),
            cpi_program_keypair,
            authority_keypair,
        );

        let cpi_instruction = Instruction::new(
            cpi_program_pubkey,
            // 1 disables CPI
            vec![1],
            vec![AccountMeta {
                pubkey: base_program_pubkey,
                is_signer: false,
                is_writable: false,
            }],
        );

        let client = ArchRpcClient::new(NODE1_ADDRESS);

        let cpi_tx = build_and_sign_transaction(
            ArchMessage::new(
                &[cpi_instruction],
                Some(authority_pubkey),
                client.get_best_block_hash().unwrap(),
            ),
            vec![authority_keypair.clone()],
            BITCOIN_NETWORK,
        );

        let processed_transaction = send_transactions_and_wait(vec![cpi_tx.clone()])[0].clone();

        for log in processed_transaction.clone().logs {
            println!("Log: {}", log);
        }

        println!(
            "Processed transaction id : {}",
            processed_transaction.txid()
        );

        matches!(processed_transaction.status, Status::Failed(_));
    }
}
