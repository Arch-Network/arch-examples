pub const ELF_PATH: &str = "./program/target/sbf-solana-solana/release/clock_program.so";

#[cfg(test)]
mod clock_tests {
    use crate::ELF_PATH;
    use arch_program::{account::AccountMeta, clock::Clock, system_instruction};
    use arch_sdk::{
        constants::{NODE1_ADDRESS, PROGRAM_FILE_PATH},
        helper::{
            assign_ownership_to_program, generate_new_keypair, get_processed_transaction,
            init_logging, log_scenario_start, read_account_info, send_utxo,
            sign_and_send_instruction, try_deploy_program,
        },
    };
    use borsh::{BorshDeserialize, BorshSerialize};
    #[ignore]
    #[test]
    pub fn clock_test() {
        init_logging();

        log_scenario_start(
            1,
            "Program Deployment & Clock fetch",
            "Deploying the clock program, then dumping the clock data into an account",
        );

        let program_pubkey = try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Clock-test").unwrap();

        let (account_key_pair, account_pubkey, address) = generate_new_keypair();

        let (txid, vout) = send_utxo(account_pubkey);
        println!(
            "\x1b[32m Step 1/3 Successful :\x1b[0m Utxo sent for account, creation {}:{}",
            txid, vout
        );
        let (txid, _) = sign_and_send_instruction(
            system_instruction::create_account(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                account_pubkey,
            ),
            vec![account_key_pair],
        )
        .expect("signing and sending a transaction should not fail");

        println!(
            "\x1b[32m Step 2/3 Successful :\x1b[0m Account created with address, {:?}",
            account_pubkey.0
        );
        let _processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");

        assign_ownership_to_program(&program_pubkey, account_pubkey, account_key_pair);

        println!(
            "\x1b[32m Step 3/3 Successful :\x1b[0m Ownership Successfully assigned to program!"
        );

        let (txid, _) = sign_and_send_instruction(
            arch_program::instruction::Instruction {
                program_id: program_pubkey,
                accounts: vec![AccountMeta {
                    pubkey: account_pubkey,
                    is_signer: true,
                    is_writable: true,
                }],
                data: vec![],
            },
            vec![account_key_pair],
        )
        .expect("signing and sending a transaction should not fail");

        let _processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");

        let account_info = read_account_info(NODE1_ADDRESS, account_pubkey).unwrap();

        let mut account_info_data = account_info.data.as_slice();

        let account_clock = Clock::deserialize(&mut account_info_data).unwrap();

        println!("Clock data: {:?}", account_clock);
    }
}
