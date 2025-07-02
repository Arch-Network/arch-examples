use arch_program::{
    account::{AccountInfo, AccountMeta, MIN_ACCOUNT_LAMPORTS},
    bitcoin::{self, absolute::LockTime, transaction::Version, Transaction},
    entrypoint,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    instruction::Instruction,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height, invoke_signed, next_account_info,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction::create_account_with_anchor,
    transaction_to_sign::TransactionToSign,
    utxo::UtxoMeta,
};
use borsh::{BorshDeserialize, BorshSerialize};

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let vault_pda = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    assert!(payer.is_writable);
    assert!(payer.is_signer);
    assert!(vault_pda.is_writable);
    assert_eq!(vault_pda.owner, &Pubkey::system_program());
    assert_eq!(system_program.key, &Pubkey::system_program());

    let params: HelloWorldParams = borsh::from_slice(instruction_data).unwrap();
    let vault_bump_seed = params.vault_bump_seed;
    let vault_seeds = &[b"vault", payer.key.as_ref(), &[vault_bump_seed]];
    let expected_vault_pda = Pubkey::create_program_address(vault_seeds, program_id)?;
    assert_eq!(vault_pda.key, &expected_vault_pda);

    msg!("starting cpi call to create account");

    invoke_signed(
        &create_account_with_anchor(
            payer.key,
            vault_pda.key,
            MIN_ACCOUNT_LAMPORTS,
            0,
            program_id,
            params.utxo.txid().try_into().unwrap(),
            params.utxo.vout(),
        ),
        &[vault_pda.clone(), payer.clone(), system_program.clone()],
        &[&[b"vault", payer.key.as_ref(), &[vault_bump_seed]]],
    )?;

    msg!("writing program id to vault pda");

    vault_pda.realloc(32, true)?;
    vault_pda
        .data
        .borrow_mut()
        .copy_from_slice(&program_id.serialize());

    // invoke_signed(
    //     &Instruction {
    //         program_id: Pubkey::system_program(),
    //         accounts: vec![AccountMeta {
    //             pubkey: vault_pda.key.clone(),
    //             is_signer: true,
    //             is_writable: true,
    //         }],
    //         data,
    //     },
    //     &[vault_pda.clone()],
    //     &[&[b"vault", payer.key.as_ref(), &[vault_bump_seed]]],
    // )?;

    msg!("done");

    Ok(())
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct HelloWorldParams {
    pub vault_bump_seed: u8,
    pub utxo: UtxoMeta,
}
