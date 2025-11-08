use arch_program::{
    account::{AccountInfo, AccountMeta},
    bitcoin::{
        self, absolute::LockTime, transaction::Version, Address, Amount, ScriptBuf, Transaction,
        TxOut,
    },
    entrypoint,
    input_to_sign::InputToSign,
    instruction::Instruction,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height, get_clock, invoke, next_account_info,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    transaction_to_sign::TransactionToSign,
    utxo::UtxoMeta,
};
use borsh::{BorshDeserialize, BorshSerialize};
use std::str::FromStr;

entrypoint!(process_instruction);
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();
    let account = next_account_info(account_iter)?;

    let data_len = account
        .data
        .try_borrow()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .len();

    assert!(account.is_writable);
    assert!(account.is_signer);

    if data_len > 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let clock = get_clock();

    let serialized_clock = borsh::to_vec(&clock).map_err(|_e| ProgramError::InvalidAccountData)?;

    if serialized_clock.len() > data_len {
        account.realloc(serialized_clock.len(), true)?;
    }

    account
        .data
        .try_borrow_mut()
        .map_err(|_e| ProgramError::Custom(503))?
        .copy_from_slice(&serialized_clock);

    Ok(())
}
