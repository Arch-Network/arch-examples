use anyhow::Result;
use arch_program::{
    account::AccountMeta, instruction::Instruction, pubkey::Pubkey,
    system_instruction::SystemInstruction,
};
use bitcoin::key::Keypair;
use borsh::BorshSerialize;
use ebpf_counter::counter_helpers::generate_new_keypair;
use fungible_token_standard_program::{
    mint::{InitializeMintInput, MintInput},
    transfer::TransferInput,
};
use sdk::helper::send_utxo;

pub(crate) fn create_new_account_instruction() -> Result<(Keypair, Pubkey, Instruction)> {
    let (account_key_pair, account_pubkey, address) = generate_new_keypair();

    let (txid, vout) = send_utxo(account_pubkey);

    let account_creation_instruction = SystemInstruction::new_create_account_instruction(
        hex::decode(txid).unwrap().try_into().unwrap(),
        vout,
        account_pubkey,
    );

    Ok((
        account_key_pair,
        account_pubkey,
        account_creation_instruction,
    ))
}

pub(crate) fn initialize_mint_instruction(
    owner_pubkey: &Pubkey,
    mint_account: &Pubkey,
    token_program_account: &Pubkey,
    mint_input: InitializeMintInput,
) -> Result<Instruction> {
    let mut instruction_data = vec![0u8];

    mint_input
        .serialize(&mut instruction_data)
        .expect("Couldnt serialize mint input");

    let initialize_mint_instruction = Instruction {
        program_id: *token_program_account,
        accounts: vec![AccountMeta {
            pubkey: *mint_account,
            is_signer: true,
            is_writable: true,
        }],
        data: instruction_data,
    };

    Ok(initialize_mint_instruction)
}

pub(crate) fn mint_request_instruction(
    mint_account: &Pubkey,
    token_program_account: &Pubkey,
    balance_account: &Pubkey,
    balance_owner_account: &Pubkey,
    amount: u64,
) -> Result<Instruction> {
    let mint_input: MintInput = MintInput::new(amount);

    let mut instruction_data = vec![2u8];

    mint_input
        .serialize(&mut instruction_data)
        .expect("Couldnt serialize mint input");

    let mint_instruction = Instruction {
        program_id: *token_program_account,
        accounts: vec![
            AccountMeta {
                pubkey: *mint_account,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *balance_account,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *balance_owner_account,
                is_signer: true,
                is_writable: true,
            },
        ],
        data: instruction_data,
    };

    Ok(mint_instruction)
}

pub(crate) fn transfer_request_instruction(
    mint_account: &Pubkey,
    token_program_account: &Pubkey,
    sender_account: &Pubkey,
    sender_owner_account: &Pubkey,
    receiver_account: &Pubkey,
    amount: u64,
) -> Result<Instruction> {
    let transfer_input: TransferInput = TransferInput::new(amount);

    let mut instruction_data = vec![3u8];

    transfer_input
        .serialize(&mut instruction_data)
        .expect("Couldnt serialize mint input");

    let transfer_instruction = Instruction {
        program_id: *token_program_account,
        accounts: vec![
            AccountMeta {
                pubkey: *sender_owner_account,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *mint_account,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *sender_account,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *receiver_account,
                is_signer: false,
                is_writable: true,
            },
        ],
        data: instruction_data,
    };
    Ok(transfer_instruction)
}
