use arch_program::{
    account::AccountInfo, entrypoint, entrypoint::ProgramResult, log, msg, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Test basic logging
    msg!("Testing sol_log_data syscall");

    // Test logging binary data
    let test_data = b"Hello, World! This is binary data.";
    log::sol_log_data(&[test_data]);

    // Test logging different types of data
    let hex_data = b"0x1234567890abcdef";
    log::sol_log_data(&[hex_data]);

    // Test logging empty data
    log::sol_log_data(&[]);

    // Test logging large data
    let large_data = vec![0u8; 100];
    log::sol_log_data(&[&large_data]);

    // Test multi-chunk logging
    log::sol_log_data(&[b"abc", b"def"]);

    msg!("sol_log_data test completed successfully");

    Ok(())
}
