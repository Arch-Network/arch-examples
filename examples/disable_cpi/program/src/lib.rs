use arch_program::{
    account::AccountInfo, entrypoint, msg, program_error::ProgramError, pubkey::Pubkey,
    syscalls::arch_get_stack_height,
};

entrypoint!(process_instruction);

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    _accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if instruction_data.len() != 1 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let stack_depth = get_stack_height();
    
    msg!("Stack depth: {}", stack_depth);

    if stack_depth > 1 {
        msg!(
            "Stack depth is greater than 1, CPI detected {}",
            stack_depth
        );

        if instruction_data[0] == 1 {
            return Err(ProgramError::Custom(505));
        }
    }

    Ok(())
}
