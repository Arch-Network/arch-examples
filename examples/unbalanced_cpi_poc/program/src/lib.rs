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
    let account1 = next_account_info(account_info_iter)?;
    let account2 = next_account_info(account_info_iter)?;

    // increasing balance of account1
    let mut lamports = account1.try_borrow_mut_lamports()?;
    **lamports += 1000000000000000000;

    invoke_signed(
        &Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: account1.key.clone(),
                is_signer: true,
                is_writable: true,
            }],
            data: vec![],
        },
        &[account1.clone()],
        &[],
    )?;

    msg!("done");

    Ok(())
}
