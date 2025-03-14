use anyhow::{anyhow, Result};
use arch_program::pubkey::Pubkey;
use arch_program::utxo::UtxoMeta;
use arch_test_sdk::helper::{
    prepare_fees, prepare_fees_with_extra_utxo, read_account_info, send_utxo,
};
use borsh::BorshDeserialize;

use crate::counter_instructions::CounterData;

pub const DEFAULT_LOG_LEVEL: &str = "info";

pub(crate) fn get_account_counter(account_pubkey: &Pubkey) -> Result<CounterData> {
    let account_info = read_account_info(*account_pubkey);

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
