use arch_program::account::AccountMeta;
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::utxo::UtxoMeta;

use arch_test_sdk::helper::{
    assign_ownership_to_program, create_account, read_account_info, sign_and_send_instruction,
};
use bitcoin::key::Keypair;
use borsh::{BorshDeserialize, BorshSerialize};

use anyhow::{anyhow, Result};
use tracing::{debug, error};

pub(crate) fn start_new_counter(
    program_pubkey: &Pubkey,
    step: u16,
    initial_value: u16,
) -> Result<(Pubkey, Keypair)> {
    let (account_key_pair, account_pubkey, address) = create_account();

    println!(
        "\x1b[32m Step 1/3 Successful :\x1b[0m Account created with address, {:?}",
        account_pubkey.0
    );

    assign_ownership_to_program(*program_pubkey, account_pubkey, account_key_pair);

    println!("\x1b[32m Step 2/3 Successful :\x1b[0m Ownership Successfully assigned to program!");

    let serialized_counter_input = borsh::to_vec(&CounterInput {
        instruction: CounterInstruction::InitializeCounter(1, 1),
        anchoring: None,
        should_return_err: false,
        should_panic: false,
        add_output: None,
    })
    .unwrap();

    let txid = sign_and_send_instruction(
        vec![Instruction {
            program_id: *program_pubkey,
            accounts: vec![AccountMeta {
                pubkey: account_pubkey,
                is_signer: true,
                is_writable: true,
            }],
            data: serialized_counter_input,
        }],
        vec![account_key_pair],
    );

    let account_info = read_account_info(account_pubkey);

    let mut account_info_data = account_info.data.as_slice();

    let account_counter = CounterData::deserialize(&mut account_info_data).unwrap();

    if account_counter != CounterData::new(initial_value, step) {
        error!("Account content different from provided initial step and initial value !");

        debug!("Account info found within account {:?}", account_info);

        return Err(anyhow!("Account content after initialization is wrong !"));
    }

    println!("\x1b[32m Step 3/3 Successful :\x1b[0m Counter succesfully initialized \x1b[1m\x1B[34mCounter Data : Step {} ======= Value {}\x1b[0m",account_counter.current_step, account_counter.current_value);

    //print_title("COUNTER INITIALIZATION : OK !", 5);

    Ok((account_pubkey, account_key_pair))
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum CounterInstruction {
    InitializeCounter(u16, u16),
    IncreaseCounter,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct CounterData {
    pub current_value: u16,
    pub current_step: u16,
}

impl CounterData {
    pub fn new(current_value: u16, current_step: u16) -> Self {
        CounterData {
            current_value,
            current_step,
        }
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CounterInput {
    pub instruction: CounterInstruction,
    pub anchoring: Option<(UtxoMeta, Vec<u8>, bool)>,
    pub should_return_err: bool,
    pub should_panic: bool,
    pub add_output: Option<u64>,
}

pub(crate) fn get_counter_increase_instruction(
    program_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    should_return_err: bool,
    should_panic: bool,
    anchoring: Option<(UtxoMeta, Vec<u8>, bool)>,
    add_output: Option<u64>,
) -> Instruction {
    let serialized_counter_input = borsh::to_vec(&CounterInput {
        instruction: CounterInstruction::IncreaseCounter,
        anchoring,
        should_return_err,
        should_panic,
        add_output,
    })
    .unwrap();

    Instruction {
        program_id: *program_pubkey,
        accounts: vec![AccountMeta {
            pubkey: *account_pubkey,
            is_signer: true,
            is_writable: true,
        }],
        data: serialized_counter_input,
    }
}
