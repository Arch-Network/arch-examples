use arch_program::{
    account::{AccountInfo, AccountMeta},
    bitcoin::{self, absolute::LockTime, transaction::Version, Transaction},
    entrypoint,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    instruction::Instruction,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height, invoke_signed, next_account_info,
        set_transaction_to_sign,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
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
        &system_instruction::create_account(
            params.utxo.txid().try_into().unwrap(),
            params.utxo.vout(),
            vault_pda.key.clone(),
        ),
        &[vault_pda.clone()],
        &[&[b"vault", payer.key.as_ref(), &[vault_bump_seed]]],
    )?;

    msg!("starting cpi call to write bytes");

    let mut data = vec![3];
    data.extend(program_id.serialize());

    invoke_signed(
        &Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: vault_pda.key.clone(),
                is_signer: true,
                is_writable: true,
            }],
            data,
        },
        &[vault_pda.clone()],
        &[&[b"vault", payer.key.as_ref(), &[vault_bump_seed]]],
    )?;

    msg!("done");

    Ok(())
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct HelloWorldParams {
    pub vault_bump_seed: u8,
    pub utxo: UtxoMeta,
}
