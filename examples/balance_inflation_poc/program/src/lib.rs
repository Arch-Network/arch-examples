use arch_program::{
    account::AccountInfo,
    entrypoint,
    program::next_account_info,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();
    let account_copy_1 = next_account_info(account_iter)?;
    let _ignore = next_account_info(account_iter)?;
    let account_copy_2 = next_account_info(account_iter)?;
    
    // Assert we're doubling the balance of the same account
    assert_eq!(account_copy_1.key, account_copy_2.key);

    **account_copy_1.lamports.borrow_mut() *= 2;
    **account_copy_2.lamports.borrow_mut() = 0;

    Ok(())
}
