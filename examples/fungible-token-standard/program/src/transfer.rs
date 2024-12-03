use arch_program::{account::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{mint::TokenMintDetails, token_account::TokenBalance};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TransferInput {
    pub amount: u64,
}
impl TransferInput {
    pub fn new(amount: u64) -> Self {
        TransferInput { amount }
    }
}

pub fn transfer_tokens(
    owner_account: &AccountInfo<'_>,
    mint_account: &AccountInfo<'_>,
    sender_account: &AccountInfo<'_>,
    receiver_account: &AccountInfo<'_>,
    program_id: &Pubkey,
    transfer_input: TransferInput,
) -> Result<(), ProgramError> {
    /* ------------------------- Sender account checks ------------------------- */
    let mut sender_token_balance_data = sender_account
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let mut sender_token_balance = TokenBalance::deserialize(&mut &sender_token_balance_data[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if sender_account.owner != program_id {
        return Err(ProgramError::Custom(501));
    };

    if sender_token_balance.mint_account != mint_account.key.serialize() {
        return Err(ProgramError::Custom(503));
    }

    if sender_token_balance.owner != owner_account.key.serialize() {
        return Err(ProgramError::Custom(502));
    }

    /* ------------------------- Receiver account checks ------------------------- */

    let mut receiver_token_balance_data = receiver_account
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let mut receiver_token_balance =
        TokenBalance::deserialize(&mut &receiver_token_balance_data[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if receiver_account.owner != program_id {
        return Err(ProgramError::Custom(505));
    };

    if receiver_token_balance.mint_account != mint_account.key.serialize() {
        return Err(ProgramError::Custom(506));
    }

    /* --------------------------- MINT ACCOUNT CHECKS -------------------------- */

    let mint_data = mint_account
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let mint_details = TokenMintDetails::deserialize(&mut &mint_data[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if mint_account.owner != program_id {
        return Err(ProgramError::Custom(504));
    }
    /* -------------------------- OWNER ACCOUNT CHECKS -------------------------- */
    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    /* -------------------------------- EXECUTION ------------------------------- */
    sender_token_balance.decrease_balance(transfer_input.amount, &mint_details)?;

    receiver_token_balance.increase_balance(transfer_input.amount, &mint_details);

    /* -------------------------- UPDATE SENDER BALANCE ------------------------- */

    let new_serialized_sender_balance = borsh::to_vec(&sender_token_balance).unwrap();

    if new_serialized_sender_balance.len() > sender_token_balance_data.len() {
        sender_account.realloc(new_serialized_sender_balance.len(), true)?;
    }

    /* ------------------------- UPDATE RECEIVER BALANCE ------------------------ */

    let new_serialized_receiver_balance = borsh::to_vec(&receiver_token_balance).unwrap();

    if new_serialized_receiver_balance.len() > receiver_token_balance_data.len() {
        receiver_account.realloc(new_serialized_receiver_balance.len(), true)?;
    }

    receiver_token_balance_data.copy_from_slice(&new_serialized_receiver_balance);

    sender_token_balance_data.copy_from_slice(&new_serialized_sender_balance);

    Ok(())
}
