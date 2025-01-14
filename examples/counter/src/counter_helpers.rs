use anyhow::{anyhow, Result};
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction::SystemInstruction;
use arch_program::utxo::UtxoMeta;
use arch_sdk::constants::NODE1_ADDRESS;
use arch_sdk::helper::{
    get_processed_transaction, prepare_fees, prepare_fees_with_extra_utxo, read_account_info,
    send_utxo, sign_and_send_instruction,
};
use bitcoin::key::Keypair;
use borsh::BorshDeserialize;

use crate::counter_instructions::CounterData;

pub(crate) fn get_account_counter(account_pubkey: &Pubkey) -> Result<CounterData> {
    let account_info = read_account_info(NODE1_ADDRESS, *account_pubkey)
        .map_err(|e| anyhow!(format!("Error reading account content {}", e.to_string())))?;

    let mut account_info_data = account_info.data.as_slice();

    let account_counter = CounterData::deserialize(&mut account_info_data)
        .map_err(|e| anyhow!(format!("Error corrupted account data {}", e.to_string())))?;

    Ok(account_counter)
}

pub(crate) fn generate_anchoring_psbt(account_pubkey: &Pubkey) -> (UtxoMeta, Vec<u8>) {
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
