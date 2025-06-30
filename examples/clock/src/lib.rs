pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/clock_program.so";

#[cfg(test)]
mod clock_tests {
    use crate::ELF_PATH;
    use arch_program::{account::AccountMeta, clock::Clock};
    use arch_test_sdk::{
        constants::PROGRAM_FILE_PATH,
        helper::{
            assign_ownership_to_program, create_account, deploy_program, read_account_info,
            sign_and_send_instruction,
        },
        logging::{init_logging, log_scenario_start},
    };
    use borsh::BorshDeserialize;
    #[ignore]
    #[test]
    pub fn clock_test() {
        init_logging();

        log_scenario_start(
            1,
            "Program Deployment & Clock fetch",
            "Deploying the clock program, then dumping the clock data into an account",
        );

        let program_pubkey = deploy_program(
            ELF_PATH.to_string(),
            PROGRAM_FILE_PATH.to_string(),
            "Clock-test".to_string(),
        );

        let (account_key_pair, account_pubkey, address) = create_account();

        println!(
            "\x1b[32m Step 1/2 Successful :\x1b[0m Account created with address, {:?}",
            account_pubkey.0
        );

        assign_ownership_to_program(program_pubkey, account_pubkey, account_key_pair);

        println!(
            "\x1b[32m Step 2/2 Successful :\x1b[0m Ownership Successfully assigned to program!"
        );

        let instruction = arch_program::instruction::Instruction {
            program_id: program_pubkey,
            accounts: vec![AccountMeta {
                pubkey: account_pubkey,
                is_signer: true,
                is_writable: true,
            }],
            data: vec![],
        };

        let txid = sign_and_send_instruction(vec![instruction], vec![account_key_pair]);

        let account_info = read_account_info(account_pubkey);

        let mut account_info_data = account_info.data.as_slice();

        let account_clock = Clock::deserialize(&mut account_info_data).unwrap();

        println!("Clock data: {:?}", account_clock);
    }
}
