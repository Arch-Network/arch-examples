use arch_program::{
    account::{AccountInfo, MIN_ACCOUNT_LAMPORTS},
    bitcoin::{self, absolute::LockTime, transaction::Version, Transaction},
    entrypoint,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    msg,
    program::{invoke, next_account_info, set_transaction_to_sign},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    utxo::UtxoMeta,
    system_program::SYSTEM_PROGRAM_ID,
};
use borsh::{BorshDeserialize, BorshSerialize};

/// State structure to keep track of accounts created by this factory
/// Uses Borsh serialization for efficient storage and retrieval
#[derive(BorshSerialize, BorshDeserialize)]
pub struct FactoryState {
    pub total_accounts_created: u64, // Counter for total accounts
    pub last_account_created: Option<Pubkey>, // Most recently created account
}

/// Parameters required to create a new account
/// Passed in the instruction data when calling this program
#[derive(BorshSerialize, BorshDeserialize)]
pub struct CreateAccountParams {
    pub name: String,    // Identifier for the account
    pub utxo: UtxoMeta,  // UTXO information for funding
    pub tx_hex: Vec<u8>, // Bitcoin transaction for fees
}

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Step 1: Get account iterators for the accounts we need to work with
    let account_iter = &mut accounts.iter();

    // Get references to the factory state account and the new account being created
    let factory_state_account = next_account_info(account_iter)?;
    let new_account = next_account_info(account_iter)?;

    let payer = next_account_info(account_iter)?;

    // Step 2: Deserialize the instruction parameters
    let params: CreateAccountParams = borsh::from_slice(instruction_data).map_err(map_io_error)?;
    let fees_tx: Transaction = bitcoin::consensus::deserialize(&params.tx_hex).unwrap();

    // Step 3: Create the new account using Cross-Program Invocation (CPI)
    // This calls the system program to actually create the account
    invoke(
        &system_instruction::create_account_with_anchor(
            &payer.key,
            &new_account.key,
            MIN_ACCOUNT_LAMPORTS,
            0,
            &SYSTEM_PROGRAM_ID,
            params.utxo.txid().try_into().unwrap(),
            params.utxo.vout(),
        ),
        &[payer.clone(), new_account.clone()],
    )?;

    // Step 4: Update or initialize the factory state
    let mut state = if factory_state_account.data_is_empty() {
        // If this is the first account, initialize with default values
        FactoryState {
            total_accounts_created: 0,
            last_account_created: None,
        }
    } else {
        // Otherwise, load existing state
        borsh::from_slice(&factory_state_account.data.borrow()).map_err(map_io_error)?
    };

    // Update the state with new account information
    state.total_accounts_created += 1;
    state.last_account_created = Some(*new_account.key);

    // Step 5: Save the updated state back to storage
    factory_state_account.realloc(borsh::to_vec(&state).map_err(map_io_error)?.len(), true)?;
    factory_state_account
        .data
        .borrow_mut()
        .copy_from_slice(&borsh::to_vec(&state).map_err(map_io_error)?);

    // Step 6: Prepare and sign the Bitcoin transaction
    // Create a new transaction with necessary parameters
    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![],
        output: vec![],
    };
    // Add state transition and fee information
    add_state_transition(&mut tx, factory_state_account);
    tx.input.push(fees_tx.input[0].clone());

    // Create the transaction signing request
    let inputs = [InputToSign::Sign {
        index: 0,
        signer: factory_state_account.key.clone(),
    }];

    // Log the successful account creation
    msg!(
        "Created new account: {}. Total accounts created: {}",
        params.name,
        state.total_accounts_created
    );

    // Step 7: Queue the transaction for signing
    set_transaction_to_sign(accounts, &tx, &inputs)?;

    Ok(())
}

// Register the entry point for our program
entrypoint!(process_instruction);

// Add this helper function at the top of the file, after the imports
fn map_io_error(error: std::io::Error) -> ProgramError {
    msg!("IO Error: {}", error);
    ProgramError::Custom(0x1) // Using a custom error code for IO errors
}
