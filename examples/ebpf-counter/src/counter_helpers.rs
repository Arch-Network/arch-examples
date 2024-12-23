use anyhow::{anyhow, Result};
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction::SystemInstruction;
use arch_program::utxo::UtxoMeta;
use arch_sdk::constants::{BITCOIN_NETWORK, NODE1_ADDRESS};
use arch_sdk::helper::{
    get_processed_transaction, prepare_fees, prepare_fees_with_extra_utxo, read_account_info,
    send_utxo, sign_and_send_instruction,
};
use bitcoin::key::{Keypair, UntweakedKeypair};
use bitcoin::XOnlyPublicKey;
use bitcoin::{address::Address, secp256k1::Secp256k1};
use borsh::BorshDeserialize;
use rand_core::OsRng;

use crate::counter_instructions::CounterData;

pub const DEFAULT_LOG_LEVEL: &str = "info";

pub fn print_title(title: &str, color: u8) {
    let termsize::Size { rows: _, cols } =
        termsize::get().unwrap_or(termsize::Size { rows: 24, cols: 80 });
    let term_width = usize::from(cols);

    let color_code = match color {
        1 => 34, // Blue
        2 => 33, // Yellow
        3 => 31, // Red
        4 => 36, // Cyan
        _ => 32, // Green (default)
    };

    let start_format = format!("\x1b[1m\x1b[{}m", color_code);
    let reset_format = "\x1b[0m";

    let line = format!("===== {} ", title);
    let remaining_width = term_width.saturating_sub(line.len());
    let dashes = "=".repeat(remaining_width);

    println!("{}{}{}{}", start_format, line, dashes, reset_format);
}

pub fn log_scenario_start(scenario_index: u16, scenario_title: &str, scenario_description: &str) {
    println!("\n\n\n");

    // Print header separator
    print_title("", 1); // Blue separator line

    // Print scenario title
    println!(
        "\x1b[1m\x1b[36m===== Scenario {} : \x1b[0m\x1b[1m {} \x1b[36m=====\x1b[0m",
        scenario_index, scenario_title
    );

    print_title("", 1); // Blue separator line

    // Print description section
    println!(
        "\x1b[1m\x1b[3m\x1b[36m=====\x1b[0m \x1b[1m\x1b[3m {} \x1b[0m",
        scenario_description
    );
    // Print footer separator
    print_title("", 1); // Blue separator line
}

pub fn log_scenario_end(scenario_index: u16, scenario_states: &str) {
    println!();

    // Print end separator
    print_title("", 1); // Blue separator line

    // Print scenario end message
    println!(
        "\x1b[1m\x1b[32m===== Scenario {} Finished Successfully! \x1b[0m\x1b[1m Final state: {} \x1b[32m=====\x1b[0m",
        scenario_index, scenario_states
    );

    // Print footer separator
    print_title("", 1); // Blue separator line
}

pub fn init_logging() {
    use std::{env, sync::Once};

    static INIT: Once = Once::new();

    INIT.call_once(|| {
        if env::var("RUST_LOG").is_err() {
            env::set_var("RUST_LOG", DEFAULT_LOG_LEVEL);
        }

        tracing_subscriber::fmt()
            .without_time()
            .with_file(false)
            .with_line_number(false)
            .with_env_filter(tracing_subscriber::EnvFilter::new(format!(
                "{},reqwest=off,hyper=off",
                env::var("RUST_LOG").unwrap()
            )))
            .init();
    });
}

pub fn assign_ownership_to_program(
    program_pubkey: &Pubkey,
    account_to_transfer_pubkey: Pubkey,
    current_owner_keypair: Keypair,
) {
    let mut instruction_data = vec![3];
    instruction_data.extend(program_pubkey.serialize());

    let (txid, _) = sign_and_send_instruction(
        SystemInstruction::new_assign_ownership_instruction(
            account_to_transfer_pubkey,
            *program_pubkey,
        ),
        vec![current_owner_keypair],
    )
    .expect("signing and sending a transaction should not fail");

    let _processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
        .expect("get processed transaction should not fail");
}

pub fn generate_new_keypair() -> (UntweakedKeypair, Pubkey, Address) {
    let secp = Secp256k1::new();

    let (secret_key, _public_key) = secp.generate_keypair(&mut OsRng);

    let key_pair = UntweakedKeypair::from_secret_key(&secp, &secret_key);

    let (x_only_public_key, _parity) = XOnlyPublicKey::from_keypair(&key_pair);

    let address = Address::p2tr(&secp, x_only_public_key, None, BITCOIN_NETWORK);

    let pubkey = Pubkey::from_slice(&XOnlyPublicKey::from_keypair(&key_pair).0.serialize());

    (key_pair, pubkey, address)
}

pub(crate) fn get_account_counter(account_pubkey: &Pubkey) -> Result<CounterData> {
    let account_info = read_account_info(NODE1_ADDRESS, *account_pubkey)
        .map_err(|e| anyhow!(format!("Error reading account content {}", e.to_string())))?;

    let mut account_info_data = account_info.data.as_slice();

    let account_counter = CounterData::deserialize(&mut account_info_data)
        .map_err(|e| anyhow!(format!("Error corrupted account data {}", e.to_string())))?;

    Ok(account_counter)
}

pub(crate) fn generate_anchoring(account_pubkey: &Pubkey) -> (UtxoMeta, Vec<u8>) {
    let (utxo_txid, utxo_vout) = send_utxo(*account_pubkey);

    let fees_psbt = prepare_fees();

    (
        UtxoMeta::from(
            hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
            utxo_vout,
        ),
        hex::decode(fees_psbt).unwrap(),
    )
}

pub(crate) fn generate_extra_rune(account_pubkey: &Pubkey) -> (UtxoMeta, UtxoMeta, Vec<u8>) {
    let (utxo_txid, utxo_vout) = send_utxo(*account_pubkey);

    let (rune_txid, rune_vout) = send_utxo(*account_pubkey);

    let fees_psbt = prepare_fees_with_extra_utxo(rune_txid.clone(), rune_vout);

    (
        UtxoMeta::from(
            hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
            utxo_vout,
        ),
        UtxoMeta::from(
            hex::decode(rune_txid.clone()).unwrap().try_into().unwrap(),
            rune_vout,
        ),
        hex::decode(fees_psbt).unwrap(),
    )
}
