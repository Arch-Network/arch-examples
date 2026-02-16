use arch_program::account::AccountMeta;
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::rent::minimum_rent;
use arch_program::sanitized::ArchMessage;
use arch_program::system_instruction;
use arch_program::utxo::UtxoMeta;

use arch_sdk::{
    build_and_sign_transaction, generate_new_keypair, ArchError, ArchRpcClient, BitcoinHelper,
    Config,
};
use bitcoin::key::Keypair;
use bitcoin::XOnlyPublicKey;
use borsh::{BorshDeserialize, BorshSerialize};

use tracing::{debug, error};

pub(crate) fn start_new_counter(
    program_pubkey: &Pubkey,
    step: u16,
    initial_value: u16,
    fee_payer_keypair: &Keypair,
) -> Result<(Pubkey, Keypair), ArchError> {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let fee_payer_pubkey = Pubkey::from_slice(
        &XOnlyPublicKey::from_keypair(fee_payer_keypair)
            .0
            .serialize(),
    );

    let (account_key_pair, account_pubkey, _) = generate_new_keypair(config.network);

    let helper = BitcoinHelper::new(&config);

    let (txid, vout) = helper.send_utxo(account_pubkey).unwrap();

    println!(
        "\x1b[32m Step 1/3 Successful :\x1b[0m Account created with address, {:?}",
        account_pubkey.0
    );

    let tx = build_and_sign_transaction(
        ArchMessage::new(
            &[system_instruction::create_account_with_anchor(
                &fee_payer_pubkey,
                &account_pubkey,
                //Increasing initial rent for extra data bytes we need to store
                minimum_rent(100),
                0,
                program_pubkey,
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
            )],
            Some(fee_payer_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![*fee_payer_keypair, account_key_pair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(tx).unwrap();

    let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();

    println!(
        "processed_tx for creating account : {:?} \n\n",
        processed_tx
    );

    println!("\x1b[32m Step 2/3 Successful :\x1b[0m Ownership Successfully assigned to program!");

    let serialized_counter_input = borsh::to_vec(&CounterInput {
        instruction: CounterInstruction::InitializeCounter(1, 1),
        anchoring: None,
        should_return_err: false,
        should_panic: false,
        add_output: None,
    })
    .unwrap();

    let tx = build_and_sign_transaction(
        ArchMessage::new(
            &[arch_program::instruction::Instruction {
                program_id: *program_pubkey,
                accounts: vec![
                    AccountMeta {
                        pubkey: account_pubkey,
                        is_signer: true,
                        is_writable: true,
                    },
                    AccountMeta::new(fee_payer_pubkey, true),
                ],
                data: serialized_counter_input,
            }],
            Some(fee_payer_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![*fee_payer_keypair, account_key_pair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(tx).unwrap();

    let processed_tx = client.wait_for_processed_transaction(&txid).unwrap();

    println!("processed_tx: {:?}", processed_tx);

    let account_info = client.read_account_info(account_pubkey).unwrap();

    let mut account_info_data = account_info.data.as_slice();

    let account_counter = CounterData::deserialize(&mut account_info_data).unwrap();

    if account_counter != CounterData::new(initial_value, step) {
        error!("Account content different from provided initial step and initial value !");

        debug!("Account info found within account {:?}", account_info);

        return Err(ArchError::ProgramError(
            "Account content after initialization is wrong !".to_string(),
        ));
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
    fee_payer_pubkey: &Pubkey,
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
        accounts: vec![
            AccountMeta {
                pubkey: *account_pubkey,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta::new(*fee_payer_pubkey, true),
        ],
        data: serialized_counter_input,
    }
}
