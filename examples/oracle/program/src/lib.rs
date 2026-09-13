use arch_program::{
    account::AccountInfo, entrypoint, msg, program::next_account_info,
    program_error::ProgramError, pubkey::Pubkey,
};

entrypoint!(update_data);
pub fn update_data(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();
    let oracle_account = next_account_info(account_iter)?;

    assert!(oracle_account.is_signer);
    assert_eq!(instruction_data.len(), 8);

    let data_len = oracle_account.data.try_borrow().unwrap().len();
    if instruction_data.len() > data_len {
        oracle_account.realloc(instruction_data.len(), true)?;
    }

    oracle_account
        .data
        .try_borrow_mut()
        .unwrap()
        .copy_from_slice(instruction_data);

    msg!("updated");

    Ok(())
}
