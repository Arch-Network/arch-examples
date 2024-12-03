use std::collections::HashMap;

use arch_program::{account::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::token_account::TokenBalance;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TokenMintDetails {
    owner: [u8; 32],
    pub status: MintStatus,
    pub supply: u64,             // in lowest denomination
    pub circulating_supply: u64, // in lowest denomination
    pub ticker: String,
    pub decimals: u8,
    token_metadata: HashMap<String, [u8; 32]>,
}

impl TokenMintDetails {
    pub fn new(
        input: InitializeMintInput,
        status: MintStatus,
        token_metadata: HashMap<String, [u8; 32]>,
    ) -> Self {
        TokenMintDetails {
            owner: input.owner,
            status,
            supply: input.supply,
            circulating_supply: 0,
            ticker: input.ticker,
            decimals: input.decimals,
            token_metadata,
        }
    }
}
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Eq, PartialEq)]
pub enum MintStatus {
    Ongoing,
    Finished,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct InitializeMintInput {
    owner: [u8; 32],
    supply: u64, // in lowest denomination
    ticker: String,
    decimals: u8,
}
impl InitializeMintInput {
    pub fn new(owner: [u8; 32], supply: u64, ticker: String, decimals: u8) -> Self {
        InitializeMintInput {
            owner,
            supply,
            ticker,
            decimals,
        }
    }
}

pub(crate) fn initialize_mint(
    account: &AccountInfo<'_>,
    program_id: &Pubkey,
    mint_input: InitializeMintInput,
) -> Result<(), ProgramError> {
    if !account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let mint_initial_details =
        TokenMintDetails::new(mint_input, MintStatus::Ongoing, HashMap::new());

    let serialized_mint_details = borsh::to_vec(&mint_initial_details)
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    if !serialized_mint_details.is_empty() {
        account.realloc(serialized_mint_details.len(), true)?;
    }

    account
        .data
        .try_borrow_mut()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .copy_from_slice(&serialized_mint_details);

    Ok(())
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MintInput {
    pub amount: u64,
}
impl MintInput {
    pub fn new(amount: u64) -> Self {
        MintInput { amount }
    }
}
pub fn mint_tokens(
    balance_account: &AccountInfo<'_>,
    mint_account: &AccountInfo<'_>,
    owner_account: &AccountInfo<'_>,
    program_id: &Pubkey,
    mint_input: MintInput,
) -> Result<(), ProgramError> {
    /* ------------------------- Balance account checks ------------------------- */
    let mut token_balance_data = balance_account
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let mut token_balance = TokenBalance::deserialize(&mut &token_balance_data[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if balance_account.owner != program_id {
        return Err(ProgramError::Custom(501));
    };

    if token_balance.mint_account != mint_account.key.serialize() {
        return Err(ProgramError::Custom(503));
    }

    if token_balance.owner != owner_account.key.serialize() {
        return Err(ProgramError::Custom(502));
    }

    /* --------------------------- MINT ACCOUNT CHECKS -------------------------- */

    let mut mint_data = mint_account
        .data
        .try_borrow_mut()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    let mut mint_details = TokenMintDetails::deserialize(&mut &mint_data[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if mint_account.owner != program_id {
        return Err(ProgramError::Custom(504));
    }

    if mint_details.status == MintStatus::Finished {
        return Err(ProgramError::Custom(502));
    }

    /* -------------------------- OWNER ACCOUNT CHECKS -------------------------- */
    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    /* -------------------------------- EXECUTION ------------------------------- */

    add_mint_to_circulating_supply(&mut mint_details, &mint_input)?;
    msg!("circulating_supply: {}", mint_details.circulating_supply);

    if mint_details.circulating_supply == mint_details.supply {
        mint_details.status = MintStatus::Finished;
    }

    token_balance.increase_balance(mint_input.amount, &mint_details);

    let new_serialized_token_balance = borsh::to_vec(&token_balance).unwrap();

    let new_serialized_mint_details = borsh::to_vec(&mint_details).unwrap();

    /* -------------------------- UPDATE TOKEN BALANCE -------------------------- */

    if new_serialized_token_balance.len() > token_balance_data.len() {
        balance_account.realloc(new_serialized_token_balance.len(), true)?;
    }
    token_balance_data.copy_from_slice(&new_serialized_token_balance);

    /* ---------------------------- UPDATE MINT DATA ---------------------------- */

    if new_serialized_mint_details.len() > mint_data.len() {
        mint_account.realloc(new_serialized_mint_details.len(), true)?;
    }
    mint_data.copy_from_slice(&new_serialized_mint_details);

    Ok(())
}

pub fn add_mint_to_circulating_supply(
    mint_details: &mut TokenMintDetails,
    mint_input: &MintInput,
) -> Result<(), ProgramError> {
    if mint_input.amount > mint_details.supply - mint_details.circulating_supply {
        msg!(
            "Not enough remaining supply. You can only mint up to {} tokens. You requested {}",
            mint_details.supply - mint_details.circulating_supply,
            mint_input.amount
        );
        return Err(ProgramError::InsufficientFunds); // Not enough remaining supply
    }

    mint_details.circulating_supply += mint_input.amount;

    Ok(())
}
