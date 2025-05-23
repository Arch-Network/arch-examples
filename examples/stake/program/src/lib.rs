use apl_token::instruction::{mint_to, transfer};
use arch_program::program_pack::Pack;
use arch_program::{
    account::{AccountInfo, MIN_ACCOUNT_LAMPORTS},
    entrypoint, msg,
    program::{invoke, invoke_signed, next_account_info},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction::create_account_with_anchor,
    utxo::UtxoMeta,
};
use borsh::{BorshDeserialize, BorshSerialize};

// Define the state of our stake account
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct StakeAccount {
    // The owner of this stake account
    pub owner: Pubkey,
    // The token mint that this stake account accepts
    pub token_mint: Pubkey,
    // The amount of tokens staked
    pub staked_amount: u64,
    // Timestamp when the stake was created
    pub stake_timestamp: u64,
    // Minimum time tokens must be staked (in seconds)
    pub lockup_duration: u64,
    // Accumulated rewards
    pub rewards: u64,
}

// Define our instruction types
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum StakeInstruction {
    // Initialize a new stake account
    Initialize {
        // Minimum time tokens must be staked
        lockup_duration: u64,
        // UTXO for mint account creation
        mint_utxo: UtxoMeta,
        // UTXO for stake account creation
        stake_utxo: UtxoMeta,
    },
    // Stake tokens
    Stake {
        // Amount of tokens to stake
        amount: u64,
    },
    // Unstake tokens
    Unstake {
        // Amount of tokens to unstake
        amount: u64,
    },
    // Claim rewards
    ClaimRewards,
}

// Find the stake account PDA for a given owner and token mint
pub fn find_stake_account_address(
    owner: &Pubkey,
    token_mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"stake", owner.as_ref(), token_mint.as_ref()], program_id)
}

// Program entrypoint
entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let instruction = StakeInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        StakeInstruction::Initialize {
            lockup_duration,
            mint_utxo,
            stake_utxo,
        } => process_initialize(program_id, accounts, lockup_duration, mint_utxo, stake_utxo),
        StakeInstruction::Stake { amount } => process_stake(program_id, accounts, amount),
        StakeInstruction::Unstake { amount } => process_unstake(program_id, accounts, amount),
        StakeInstruction::ClaimRewards => process_claim_rewards(program_id, accounts),
    }
}

// Initialize a new stake account
fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    lockup_duration: u64,
    mint_utxo: UtxoMeta,
    stake_utxo: UtxoMeta,
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let stake_account = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // check if token_mint is empty
    assert_eq!(token_mint.data_len(), 0);

    // create mint
    invoke(
        &create_account_with_anchor(
            owner.key,
            token_mint.key,
            MIN_ACCOUNT_LAMPORTS,
            apl_token::state::Mint::LEN as u64,
            token_program.key,
            mint_utxo
                .txid()
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
            mint_utxo.vout(),
        ),
        &[token_mint.clone(), owner.clone(), system_program.clone()],
    )?;

    invoke(
        &apl_token::instruction::initialize_mint(
            token_program.key,
            token_mint.key,
            owner.key,
            Some(owner.key),
            9,
        )?,
        &[token_mint.clone(), owner.clone(), token_program.clone()],
    )?;

    assert!(owner.is_signer);
    assert_eq!(system_program.key, &Pubkey::system_program());

    // Calculate the stake account address and verify it matches
    let (stake_account_pda, bump_seed) =
        find_stake_account_address(owner.key, token_mint.key, program_id);

    assert_eq!(stake_account_pda, *stake_account.key);

    msg!("Stake account address: {}", stake_account_pda);
    // Create the stake account
    let stake_account_seeds = &[
        b"stake",
        owner.key.as_ref(),
        token_mint.key.as_ref(),
        &[bump_seed],
    ];

    // Initialize the stake account data
    let stake_data = StakeAccount {
        owner: *owner.key,
        token_mint: *token_mint.key,
        staked_amount: 0,
        stake_timestamp: 0, // Will be set when tokens are staked
        lockup_duration,
        rewards: 0,
    };

    let serialized_stake_data =
        borsh::to_vec(&stake_data).map_err(|_| ProgramError::InvalidAccountData)?;
    msg!("Stake account data: {:?}", stake_data);

    // Create account using CPI
    invoke_signed(
        &create_account_with_anchor(
            owner.key,
            stake_account.key,
            MIN_ACCOUNT_LAMPORTS,
            serialized_stake_data.len() as u64,
            program_id,
            stake_utxo
                .txid()
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
            stake_utxo.vout(),
        ),
        &[stake_account.clone(), owner.clone()],
        &[stake_account_seeds],
    )?;

    msg!(
        "Stake account created with pubkey : {:?}",
        stake_account.key
    );

    let data_len = stake_account
        .data
        .try_borrow()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .len();

    msg!("stake account info before write : {:?}", stake_account);
    if serialized_stake_data.len() > data_len {
        stake_account.realloc(serialized_stake_data.len(), true)?;
    }

    stake_account
        .data
        .try_borrow_mut()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .copy_from_slice(&serialized_stake_data);

    msg!("Stake account initialized");
    Ok(())
}

// Stake tokens
fn process_stake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    msg!("Stake ix");
    let account_info_iter = &mut accounts.iter();

    // Get accounts
    let owner = next_account_info(account_info_iter)?;
    let stake_account = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let stake_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    assert!(owner.is_signer);
    assert_eq!(token_program.key, &apl_token::id());

    // Calculate the stake account address and verify it matches
    let (stake_account_pda, _) = find_stake_account_address(owner.key, token_mint.key, program_id);

    assert_eq!(stake_account_pda, *stake_account.key);

    // Load stake account data
    let mut stake_data = StakeAccount::try_from_slice(&stake_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    assert_eq!(stake_data.owner, *owner.key);
    assert_eq!(stake_data.token_mint, *token_mint.key);

    // Transfer tokens from user to stake account
    invoke(
        &transfer(
            token_program.key,
            user_token_account.key,
            stake_token_account.key,
            owner.key,
            &[&owner.key],
            amount,
        )?,
        &[
            user_token_account.clone(),
            stake_token_account.clone(),
            owner.clone(),
            token_program.clone(),
        ],
    )?;

    // Update stake account data
    stake_data.staked_amount += amount;
    // Set stake timestamp if this is the first stake
    if stake_data.stake_timestamp == 0 {
        // In a real implementation, you would get the current timestamp
        // For simplicity, we'll use a placeholder value
        stake_data.stake_timestamp = 1000000; // Placeholder timestamp
    }

    // Save updated stake account data
    stake_data
        .serialize(&mut *stake_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("Tokens staked: {}", amount);
    Ok(())
}

// Unstake tokens
fn process_unstake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let stake_account = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let stake_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    assert!(owner.is_signer);
    assert_eq!(token_program.key, &apl_token::id());

    // Calculate the stake account address and verify it matches
    let (stake_account_pda, bump_seed) =
        find_stake_account_address(owner.key, token_mint.key, program_id);

    assert_eq!(stake_account_pda, *stake_account.key);

    // Load stake account data
    let mut stake_data = StakeAccount::try_from_slice(&stake_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    assert_eq!(stake_data.owner, *owner.key);
    assert_eq!(stake_data.token_mint, *token_mint.key);
    assert!(stake_data.staked_amount >= amount);

    // Check if lockup period has passed
    // In a real implementation, you would get the current timestamp
    let current_timestamp = 2000000; // Placeholder timestamp
    let time_staked = current_timestamp - stake_data.stake_timestamp;

    if time_staked < stake_data.lockup_duration {
        return Err(ProgramError::Custom(100)); // Custom error for lockup period
    }

    // Transfer tokens from stake account to user
    let stake_account_seeds = &[
        b"stake",
        owner.key.as_ref(),
        token_mint.key.as_ref(),
        &[bump_seed],
    ];

    invoke_signed(
        &transfer(
            token_program.key,
            stake_token_account.key,
            user_token_account.key,
            stake_account.key,
            &[],
            amount,
        )?,
        &[
            stake_token_account.clone(),
            user_token_account.clone(),
            stake_account.clone(),
            token_program.clone(),
        ],
        &[stake_account_seeds],
    )?;

    // Update stake account data
    stake_data.staked_amount -= amount;

    // If all tokens are unstaked, reset the stake timestamp
    if stake_data.staked_amount == 0 {
        stake_data.stake_timestamp = 0;
    }

    // Save updated stake account data
    stake_data
        .serialize(&mut *stake_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("Tokens unstaked: {}", amount);
    Ok(())
}

// Claim rewards
fn process_claim_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let stake_account = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let reward_authority = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    assert!(owner.is_signer);
    assert_eq!(token_program.key, &apl_token::id());

    // Calculate the stake account address and verify it matches
    let (stake_account_pda, _) = find_stake_account_address(owner.key, token_mint.key, program_id);

    assert_eq!(stake_account_pda, *stake_account.key);

    // Load stake account data
    let mut stake_data = StakeAccount::try_from_slice(&stake_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    assert_eq!(stake_data.owner, *owner.key);

    assert_eq!(stake_data.token_mint, *token_mint.key);

    // Check if there are rewards to claim
    if stake_data.rewards == 0 {
        return Err(ProgramError::InsufficientFunds);
    }

    // Calculate rewards based on staking duration and amount
    // In a real implementation, this would be a more complex calculation
    let rewards_to_claim = stake_data.rewards;

    // Mint reward tokens to the user
    invoke(
        &mint_to(
            token_program.key,
            token_mint.key,
            user_token_account.key,
            reward_authority.key,
            &[],
            rewards_to_claim,
        )?,
        &[
            token_mint.clone(),
            user_token_account.clone(),
            reward_authority.clone(),
            token_program.clone(),
        ],
    )?;

    // Reset rewards
    stake_data.rewards = 0;

    // Save updated stake account data
    stake_data
        .serialize(&mut *stake_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("Rewards claimed: {}", rewards_to_claim);
    Ok(())
}
