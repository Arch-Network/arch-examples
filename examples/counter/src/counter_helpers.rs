use arch_program::pubkey::Pubkey;
use arch_program::utxo::UtxoMeta;
use arch_sdk::{prepare_fees, ArchError, ArchRpcClient, BitcoinHelper, Config};
use bitcoin::key::{Secp256k1, UntweakedKeypair};
use bitcoin::{Address, XOnlyPublicKey};
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

pub fn generate_new_keypair() -> (UntweakedKeypair, Pubkey, Address) {
    let secp = Secp256k1::new();

    let (secret_key, _public_key) = secp.generate_keypair(&mut OsRng);

    let key_pair = UntweakedKeypair::from_secret_key(&secp, &secret_key);

    let (x_only_public_key, _parity) = XOnlyPublicKey::from_keypair(&key_pair);

    let address = Address::p2tr(&secp, x_only_public_key, None, Config::localnet().network);

    let pubkey = Pubkey::from_slice(&XOnlyPublicKey::from_keypair(&key_pair).0.serialize());

    (key_pair, pubkey, address)
}

pub(crate) fn get_account_counter(account_pubkey: &Pubkey) -> Result<CounterData, ArchError> {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);

    let account_info = client.read_account_info(*account_pubkey).unwrap();

    let mut account_info_data = account_info.data.as_slice();

    let account_counter = CounterData::deserialize(&mut account_info_data).map_err(|e| {
        ArchError::ProgramError(format!("Error corrupted account data {}", e.to_string()))
    })?;

    Ok(account_counter)
}

pub(crate) fn generate_anchoring(account_pubkey: &Pubkey) -> (UtxoMeta, Vec<u8>) {
    let helper = BitcoinHelper::new(&Config::localnet());
    let (utxo_txid, utxo_vout) = helper.send_utxo(*account_pubkey).unwrap();

    let fees_psbt = prepare_fees();

    (
        UtxoMeta::from(
            hex::decode(utxo_txid.clone()).unwrap().try_into().unwrap(),
            utxo_vout,
        ),
        hex::decode(fees_psbt).unwrap(),
    )
}
