use arch_program::{
    account::AccountInfo,
    entrypoint,
    msg,
    program::{get_bitcoin_block_height, next_account_info},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

// Register our program's entrypoint function
entrypoint!(process_instruction);

/// Main program entrypoint. This function is called whenever someone wants
/// to interact with our Hello World program.
///
/// # Arguments
/// * `_program_id` - The public key of our program
/// * `accounts` - Array of accounts that this instruction will operate on
/// * `instruction_data` - The data passed to this instruction, containing the name
pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Get the current Bitcoin block height for reference
    let bitcoin_block_height = get_bitcoin_block_height();
    msg!("bitcoin_block_height {:?}", bitcoin_block_height);

    // Get an iterator over the accounts and get the first (and only) account
    let account_iter = &mut accounts.iter();
    let account = next_account_info(account_iter)?;

    assert!(account.is_writable);
    assert!(account.is_signer);
    msg!("account {:?}", account);

    // Deserialize the instruction data into our params struct
    let params: HelloWorldParams = borsh::from_slice(instruction_data).unwrap();

    // Create our greeting message
    let new_data = format!("Hello {}", params.name);

    // Check if we need to resize the account to fit our greeting
    let data_len = account.data.try_borrow().unwrap().len();
    if new_data.as_bytes().len() > data_len {
        account.realloc(new_data.len(), true)?;
    }

    // Store our greeting in the account's data
    account
        .data
        .try_borrow_mut()
        .unwrap()
        .copy_from_slice(new_data.as_bytes());

    Ok(())
}

/// Parameters passed to our program
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct HelloWorldParams {
    /// The name to say hello to
    pub name: String,
}
