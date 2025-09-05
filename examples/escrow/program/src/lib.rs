use arch_program::{
    account::{AccountInfo},
    entrypoint, msg,
    program::{invoke, invoke_signed, next_account_info},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction::create_account_with_anchor,
    utxo::UtxoMeta,
    rent::minimum_rent,
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MakeOffer {
    /// The bump seed for the offer's Program Derived Address
    pub offer_bump_seed: u8,
    /// The UTXO metadata associated with this offer
    pub offer_utxo: UtxoMeta,
    /// Unique identifier for the offer
    pub id: u64,
    /// Amount of token A being offered
    pub token_a_offered_amount: u64,
    /// Amount of token B wanted in exchange
    pub token_b_wanted_amount: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Offer {
    /// Unique identifier for the offer
    pub id: u64,
    /// Public key of the user who created the offer
    pub maker: Pubkey,
    /// Public key of the mint for token A
    pub token_mint_a: Pubkey,
    /// Public key of the mint for token B
    pub token_mint_b: Pubkey,
    /// Amount of token B wanted in exchange
    pub token_b_wanted_amount: u64,
    /// The bump seed for the offer's Program Derived Address
    pub bump: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum EscrowInstruction {
    /// Create a new offer to exchange tokens
    MakeOffer(MakeOffer),
    /// Accept an existing offer
    TakeOffer,
}

// Program entrypoint
entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let instruction = EscrowInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        EscrowInstruction::MakeOffer(data) => process_make_offer(program_id, accounts, data),
        EscrowInstruction::TakeOffer => process_take_offer(program_id, accounts, instruction_data),
    }
}

fn process_make_offer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: MakeOffer,
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let offer_info = next_account_info(account_info_iter)?;
    let token_mint_a = next_account_info(account_info_iter)?;
    let token_mint_b = next_account_info(account_info_iter)?;
    let maker_token_account_a = next_account_info(account_info_iter)?;
    let vault = next_account_info(account_info_iter)?;
    let maker = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let associated_token_program = next_account_info(account_info_iter)?;

    // account validations
    assert!(maker.is_writable);
    assert!(maker.is_signer);

    assert!(offer_info.is_writable);
    assert!(offer_info.data_is_empty());

    assert!(token_mint_a.is_writable);
    assert_eq!(token_mint_a.owner, &apl_token::id());

    assert!(token_mint_b.is_writable);
    assert_eq!(token_mint_b.owner, &apl_token::id());

    assert_eq!(token_program.key, &apl_token::id());
    assert_eq!(
        associated_token_program.key,
        &apl_associated_token_account::id()
    );
    assert_eq!(system_program.key, &Pubkey::system_program());

    // get params
    let params: MakeOffer = data;
    let id = params.id.to_le_bytes();

    // offer PDA seeds
    let offer_seeds = &[b"offer", maker.key.as_ref(), id.as_ref()];

    // verify the program address is correct
    let expected_offer_pda = Pubkey::find_program_address(offer_seeds, program_id);
    assert_eq!(offer_info.key, &expected_offer_pda.0);

    let offer_data = Offer {
        bump: params.offer_bump_seed,
        maker: *maker.key,
        id: params.id,
        token_b_wanted_amount: params.token_b_wanted_amount,
        token_mint_a: *token_mint_a.key,
        token_mint_b: *token_mint_b.key,
    };
    let serialized_offer_data =
        borsh::to_vec(&offer_data).map_err(|_| ProgramError::InvalidAccountData)?;

    let offer_signer_seeds = &[
        b"offer",
        maker.key.as_ref(),
        id.as_ref(),
        &[expected_offer_pda.1],
    ];

    // create offer PDA
    invoke_signed(
        &create_account_with_anchor(
            maker.key,
            offer_info.key,
            minimum_rent(serialized_offer_data.len()),
            serialized_offer_data.len() as u64,
            program_id,
            params
                .offer_utxo
                .txid()
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
            params.offer_utxo.vout(),
        ),
        &[offer_info.clone(), maker.clone()],
        &[offer_signer_seeds],
    )?;

    assert!(maker_token_account_a.is_writable);
    assert_eq!(maker_token_account_a.owner, &apl_token::id());

    let associated_account_address =
        apl_associated_token_account::get_associated_token_address_and_bump_seed(
            &offer_info.key,
            &token_mint_a.key,
            &apl_associated_token_account::id(),
        )
        .0;
    assert!(vault.is_writable);
    assert_eq!(associated_account_address, *vault.key);

    // build transfer instruction
    let transfer_ix = apl_token::instruction::transfer(
        token_program.key,
        maker_token_account_a.key,
        vault.key,
        maker.key,
        &[&maker.key],
        params.token_a_offered_amount,
    )?;

    // invoke transfer instruction
    arch_program::program::invoke(
        &transfer_ix,
        &[
            token_program.clone(),
            maker_token_account_a.clone(),
            vault.clone(),
            maker.clone(),
        ],
    )?;

    let data_len = offer_info
        .data
        .try_borrow()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .len();

    if serialized_offer_data.len() > data_len {
        offer_info.realloc(serialized_offer_data.len(), true)?;
    }

    offer_info
        .data
        .try_borrow_mut()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .copy_from_slice(&serialized_offer_data);

    Ok(())
}

fn process_take_offer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let offer_info = next_account_info(account_info_iter)?;
    let token_mint_a = next_account_info(account_info_iter)?;
    let token_mint_b = next_account_info(account_info_iter)?;
    let maker_token_account_b = next_account_info(account_info_iter)?;
    let taker_token_account_a = next_account_info(account_info_iter)?;
    let taker_token_account_b = next_account_info(account_info_iter)?;
    let vault = next_account_info(account_info_iter)?;
    let maker = next_account_info(account_info_iter)?;
    let taker = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let associated_token_program = next_account_info(account_info_iter)?;

    // account validation
    assert!(maker.is_writable);

    assert!(taker.is_writable);
    assert!(taker.is_signer);

    assert!(token_mint_a.is_writable);
    assert_eq!(token_mint_a.owner, &apl_token::id());

    assert!(token_mint_b.is_writable);
    assert_eq!(token_mint_b.owner, &apl_token::id());

    assert!(maker_token_account_b.is_writable);

    assert!(taker_token_account_a.is_writable);
    assert_eq!(taker_token_account_a.owner, &apl_token::id());

    assert!(taker_token_account_b.is_writable);
    assert_eq!(taker_token_account_b.owner, &apl_token::id());

    assert_eq!(vault.owner, &apl_token::id());

    assert_eq!(
        associated_token_program.key,
        &apl_associated_token_account::id()
    );
    assert_eq!(system_program.key, &Pubkey::system_program());

    let offer = Offer::try_from_slice(&offer_info.data.borrow()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;
    assert_eq!(&offer.maker, maker.key);
    assert_eq!(&offer.token_mint_a, token_mint_a.key);
    assert_eq!(&offer.token_mint_b, token_mint_b.key);

    let offer_signer = &[b"offer", maker.key.as_ref(), &offer.id.to_le_bytes()];

    let offer_key = Pubkey::find_program_address(offer_signer, program_id).0;
    assert_eq!(*offer_info.key, offer_key);

    let vault_amount_a = apl_token::state::Account::unpack(&vault.data.borrow())?.amount;
    let taker_amount_a_before_transfer =
        apl_token::state::Account::unpack(&taker_token_account_a.data.borrow())?.amount;

    invoke(
        &apl_token::instruction::transfer(
            token_program.key,
            taker_token_account_b.key,
            maker_token_account_b.key,
            taker.key,
            &[&taker.key],
            offer.token_b_wanted_amount,
        )?,
        &[
            taker_token_account_b.clone(),
            maker_token_account_b.clone(),
            taker.clone(),
            token_program.clone(),
        ],
    )?;

    let offer_signer_seeds = &[
        b"offer",
        maker.key.as_ref(),
        &offer.id.to_le_bytes(),
        &[offer.bump],
    ];

    invoke_signed(
        &apl_token::instruction::transfer(
            token_program.key,
            vault.key,
            taker_token_account_a.key,
            offer_info.key,
            &[offer_info.key, taker.key],
            vault_amount_a,
        )?,
        &[
            token_mint_a.clone(),
            vault.clone(),
            taker_token_account_a.clone(),
            offer_info.clone(),
            taker.clone(),
            token_program.clone(),
        ],
        &[offer_signer_seeds],
    )?;

    let taker_amount_a =
        apl_token::state::Account::unpack(&taker_token_account_a.data.borrow())?.amount;
    let maker_amount_b =
        apl_token::state::Account::unpack(&maker_token_account_b.data.borrow())?.amount;

    assert_eq!(
        taker_amount_a,
        taker_amount_a_before_transfer + vault_amount_a
    );
    assert_eq!(
        maker_amount_b,
        taker_amount_a_before_transfer + offer.token_b_wanted_amount
    );

    invoke_signed(
        &apl_token::instruction::close_account(
            token_program.key,
            vault.key,
            taker.key,
            offer_info.key,
            &[],
        )?,
        &[vault.clone(), taker.clone(), offer_info.clone()],
        &[offer_signer_seeds],
    )?;

    offer_info.realloc(0, true)?;

    Ok(())
}
