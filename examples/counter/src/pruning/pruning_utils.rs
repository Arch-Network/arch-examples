use std::{thread::sleep, time::Duration};

use arch_program::{pubkey::Pubkey, utxo::UtxoMeta};
use arch_sdk::{ArchRpcClient, Result};

pub fn wait_for_blocks(client: &ArchRpcClient, num_blocks: u64) {
    let start_height = client
        .get_block_count()
        .expect("failed to get initial block count");
    let target_height = start_height + num_blocks;

    println!(
        "Waiting for {} blocks, start height: {}",
        num_blocks, start_height
    );
    loop {
        let current_height = client.get_block_count().expect("failed to get block count");

        if current_height >= target_height {
            println!(
                "Done waiting for {} blocks, current height: {}",
                num_blocks, current_height
            );

            break;
        }

        sleep(Duration::from_millis(200));
    }
}

pub fn get_account_utxo(client: &ArchRpcClient, account_pubkey: &Pubkey) -> Result<UtxoMeta> {
    let account_info = client.read_account_info(*account_pubkey).unwrap();
    let split_utxo: Vec<String> = account_info
        .utxo
        .split(":")
        .map(|s| s.to_string())
        .collect();

    let utxo = UtxoMeta::from(
        hex::decode(split_utxo[0].clone())
            .unwrap()
            .try_into()
            .unwrap(),
        split_utxo[1].parse::<u32>().unwrap(),
    );
    Ok(utxo)
}
