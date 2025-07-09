use arch_program::{
    account::{next_account_info, AccountInfo},
    entrypoint,
    instruction::Instruction,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();
    let program_account = next_account_info(account_iter)?;

    let result = invoke_signed(
        &Instruction::new(*program_account.key, vec![instruction_data[0]], vec![]),
        &[],
        &[],
    );

    if result.is_err() {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(())
}
