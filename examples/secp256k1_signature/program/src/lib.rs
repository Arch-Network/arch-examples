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
    program::{get_account_script_pubkey, get_bitcoin_block_height, invoke, next_account_info},
    program_error::ProgramError,
    pubkey::Pubkey,
    sol_secp256k1_recover::secp256k1_recover,
    system_instruction,
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
    let input_signature: Secp256k1Signature =
        borsh::from_slice(instruction_data).map_err(|_e| ProgramError::InvalidArgument)?;
    msg!("Received pubkey  {:?}", input_signature.pubkey);
    for recovery_id in 0..4 {
        match secp256k1_recover(
            &input_signature.message_hash,
            recovery_id,
            &input_signature.signature,
        ) {
            Ok(pubkey) => {
                if pubkey.0 == input_signature.pubkey {
                    msg!("Signature matches Pubkey !");
                    return Ok(());
                }
            }
            _ => {}
        };
    }
    Err(ProgramError::Custom(1))
}
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
struct Secp256k1Signature {
    pub pubkey: [u8; 64],
    pub signature: [u8; 64],
    pub message_hash: [u8; 32],
}
