use arch_program::{
    account::AccountInfo,
    bitcoin::{self, absolute::LockTime, transaction::Version, Transaction},
    entrypoint,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    keccak::hash as keccak256,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height, next_account_info,
        set_transaction_to_sign,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

// Register our program's entrypoint function
entrypoint!(process_instruction);

/// Main program entrypoint. This function demonstrates various Keccak256 hashing capabilities
///
/// # Arguments
/// * `_program_id` - The public key of our program
/// * `accounts` - Array of accounts that this instruction will operate on
/// * `instruction_data` - The data passed to this instruction, containing test parameters
pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    msg!("Keccak256 Hash Example Program - Starting execution");

    // Get the current Bitcoin block height for reference
    let bitcoin_block_height = get_bitcoin_block_height();
    msg!("Current Bitcoin block height: {}", bitcoin_block_height);

    // Get an iterator over the accounts and get the first account
    let account_iter = &mut accounts.iter();
    let account = next_account_info(account_iter)?;

    assert!(account.is_writable);
    assert!(account.is_signer);
    msg!("Account: {:?}", account.key);

    // Deserialize the instruction data into our params struct
    let params: Keccak256HashParams =
        borsh::from_slice(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("Received test type: {:?}", params.test_type);

    // Perform different hash tests based on the test type
    let hash_result = match params.test_type {
        TestType::SingleInput => {
            msg!("Testing single input hash");
            test_single_input_hash(&params.data)
        }
        TestType::MultipleInputs => {
            msg!("Testing multiple inputs hash");
            test_multiple_inputs_hash(&params.data, &params.additional_data)
        }
        TestType::EmptyInput => {
            msg!("Testing empty input hash");
            test_empty_input_hash()
        }
        TestType::LargeInput => {
            msg!("Testing large input hash");
            test_large_input_hash(&params.data)
        }
        TestType::KnownVectors => {
            msg!("Testing known test vectors");
            test_known_vectors()
        }
    }?;

    // Store the result in the account data
    let result_data = Keccak256Result {
        hash: hash_result,
        test_type: params.test_type,
        input_size: params.data.len() as u32,
        success: true,
    };

    let serialized_result = borsh::to_vec(&result_data)
        .map_err(|_| ProgramError::BorshIoError("Failed to serialize result".to_string()))?;

    // Resize account if needed
    if serialized_result.len() > account.data.try_borrow().unwrap().len() {
        account.realloc(serialized_result.len(), true)?;
    }

    // Store the result
    account
        .data
        .try_borrow_mut()
        .unwrap()
        .copy_from_slice(&serialized_result);

    msg!("Keccak256 hash result stored successfully");

    // Handle the Bitcoin transaction if provided
    if !params.tx_hex.is_empty() {
        handle_bitcoin_transaction(account, &params.tx_hex)?;
    }

    msg!("Keccak256 Hash Example Program - Execution completed successfully");
    Ok(())
}

/// Test hashing a single input
fn test_single_input_hash(data: &[u8]) -> Result<[u8; 32], ProgramError> {
    msg!("Hashing {} bytes of data", data.len());

    let hash = keccak256(data);
    msg!("Hash result: {:?}", hash);

    Ok(hash.0)
}

/// Test hashing multiple separate inputs
fn test_multiple_inputs_hash(
    data1: &[u8],
    data2: &Option<Vec<u8>>,
) -> Result<[u8; 32], ProgramError> {
    let empty_vec = vec![];
    let data2 = data2.as_ref().unwrap_or(&empty_vec);
    msg!("Hashing first input: {} bytes", data1.len());
    msg!("Hashing second input: {} bytes", data2.len());

    // Hash each input separately and then hash the concatenated hashes
    let hash1 = keccak256(data1);
    let hash2 = keccak256(data2);

    msg!("First hash: {:?}", hash1.0);
    msg!("Second hash: {:?}", hash2.0);

    // Combine the hashes and hash again
    let mut combined = Vec::new();
    combined.extend_from_slice(&hash1.0);
    combined.extend_from_slice(&hash2.0);

    let final_hash = keccak256(&combined);
    msg!("Combined hash: {:?}", final_hash.0);

    Ok(final_hash.0)
}

/// Test hashing empty input
fn test_empty_input_hash() -> Result<[u8; 32], ProgramError> {
    msg!("Hashing empty input");

    let empty_data = &[];
    let hash = keccak256(empty_data);
    msg!("Empty input hash: {:?}", hash.0);

    // Known Keccak256 hash of empty string
    let expected = [
        0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03,
        0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85,
        0xa4, 0x70,
    ];

    if hash.0 == expected {
        msg!("Empty input hash matches expected value ✓");
    } else {
        msg!("Warning: Empty input hash does not match expected value");
    }

    Ok(hash.0)
}

/// Test hashing large input
fn test_large_input_hash(data: &[u8]) -> Result<[u8; 32], ProgramError> {
    msg!("Testing large input hash with {} bytes", data.len());

    // Create a larger dataset by repeating the input data
    let mut large_data = Vec::new();
    for i in 0u32..10 {
        large_data.extend_from_slice(data);
        let i_bytes = i.to_le_bytes();
        large_data.extend_from_slice(&i_bytes);
    }

    msg!("Generated large dataset: {} bytes", large_data.len());

    let hash = keccak256(&large_data);
    msg!("Large input hash: {:?}", hash.0);

    Ok(hash.0)
}

/// Test known test vectors
fn test_known_vectors() -> Result<[u8; 32], ProgramError> {
    msg!("Testing known Keccak256 test vectors");

    // Test vector 1: "abc"
    let test1 = b"abc";
    let hash1 = keccak256(test1);
    let expected1 = [
        0x4e, 0x03, 0x65, 0x7a, 0xea, 0x45, 0xa9, 0x4f, 0xc7, 0xd4, 0x7b, 0xa8, 0x26, 0xc8, 0xd6,
        0x67, 0xc0, 0xd1, 0xe6, 0xe3, 0x3a, 0x64, 0xa0, 0x36, 0xec, 0x44, 0xf5, 0x8f, 0xa1, 0x2d,
        0x6c, 0x45,
    ];

    if hash1.0 == expected1 {
        msg!("Test vector 'abc' matches expected value ✓");
    } else {
        msg!("Warning: Test vector 'abc' does not match expected value");
        msg!("Expected: {:?}", expected1);
        msg!("Got:      {:?}", hash1.0);
    }

    // Test vector 2: "The quick brown fox jumps over the lazy dog"
    let test2 = b"The quick brown fox jumps over the lazy dog";
    let hash2 = keccak256(test2);
    let expected2 = [
        0x4d, 0x74, 0x1b, 0x6f, 0x1e, 0xb2, 0x9c, 0xb2, 0xa9, 0xb9, 0x91, 0x1c, 0x82, 0xf5, 0x6f,
        0xa8, 0xd7, 0x3b, 0x04, 0x95, 0x9d, 0x3d, 0x9d, 0x22, 0x28, 0x95, 0xdf, 0x6c, 0x0b, 0x28,
        0xaa, 0x15,
    ];

    if hash2.0 == expected2 {
        msg!("Test vector 'fox' matches expected value ✓");
    } else {
        msg!("Warning: Test vector 'fox' does not match expected value");
        msg!("Expected: {:?}", expected2);
        msg!("Got:      {:?}", hash2.0);
    }

    // Return the last hash as result
    Ok(hash2.0)
}

/// Handle Bitcoin transaction processing
fn handle_bitcoin_transaction(account: &AccountInfo, tx_hex: &[u8]) -> Result<(), ProgramError> {
    msg!("Processing Bitcoin transaction");

    // Deserialize the Bitcoin transaction
    let fees_tx: Transaction =
        bitcoin::consensus::deserialize(tx_hex).map_err(|_| ProgramError::InvalidArgument)?;

    // Get the script pubkey for this account
    let script_pubkey = get_account_script_pubkey(account.key);
    msg!("Account script pubkey: {:?}", script_pubkey);

    // Create a new Bitcoin transaction for our state transition
    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![],
        output: vec![],
    };

    // Add the state transition and fee information
    add_state_transition(&mut tx, account);
    tx.input.push(fees_tx.input[0].clone());

    msg!("Transaction prepared for signing");

    let inputs = [InputToSign {
        index: 0,
        signer: account.key.clone(),
    }];

    // Submit the transaction for signing
    set_transaction_to_sign(&[account.clone()], &tx, &inputs)?;

    Ok(())
}

/// Parameters for Keccak256 hash testing
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Keccak256HashParams {
    /// Type of test to perform
    pub test_type: TestType,
    /// Primary input data
    pub data: Vec<u8>,
    /// Additional data for multi-input tests
    pub additional_data: Option<Vec<u8>>,
    /// Bitcoin transaction for fees (optional)
    pub tx_hex: Vec<u8>,
}

/// Types of hash tests to perform
#[derive(Debug, Clone, Copy, BorshSerialize, BorshDeserialize)]
pub enum TestType {
    /// Hash a single input
    SingleInput,
    /// Hash multiple separate inputs
    MultipleInputs,
    /// Hash empty input
    EmptyInput,
    /// Hash large input
    LargeInput,
    /// Test known test vectors
    KnownVectors,
}

/// Result of Keccak256 hash operation
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Keccak256Result {
    /// The resulting hash
    pub hash: [u8; 32],
    /// Type of test performed
    pub test_type: TestType,
    /// Size of input data
    pub input_size: u32,
    /// Whether the operation was successful
    pub success: bool,
}
