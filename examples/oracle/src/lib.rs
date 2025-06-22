/// Running Tests
#[cfg(test)]
mod tests {
    use arch_program::{
        account::AccountMeta, instruction::Instruction, pubkey::Pubkey,
        system_instruction::SystemInstruction, utxo::UtxoMeta,
    };
    use arch_test_sdk::constants::BITCOIN_NETWORK;
    use bitcoincore_rpc::{Auth, Client};
    use common::constants::*;

    use borsh::{BorshDeserialize, BorshSerialize};
    use common::helper::*;
    use common::models::*;
    use serde_json::Value;
    use serial_test::serial;
    use std::fs;
    use std::str::FromStr;
    use std::thread;

    #[test]
    fn test_deploy_call() {
        println!("{:?}", 10044_u64.to_le_bytes());
        println!("{:?}", 10881_u64.to_le_bytes());

        let rpc = Client::new(
            "https://bitcoin-node.dev.aws.archnetwork.xyz:18443/wallet/testwallet",
            Auth::UserPass(
                "bitcoin".to_string(),
                "428bae8f3c94f8c39c50757fc89c39bc7e6ebc70ebf8f618".to_string(),
            ),
        )
        .unwrap();

        let (program_keypair, program_pubkey) =
            with_secret_key_file(PROGRAM_FILE_PATH).expect("getting caller info should not fail");

        let (caller_keypair, caller_pubkey) =
            with_secret_key_file(CALLER_FILE_PATH).expect("getting caller info should not fail");

        let (txid, vout) = send_utxo(program_pubkey.clone());
        println!("{}:{} {:?}", txid, vout, hex::encode(program_pubkey));

        let (txid, instruction_hash) = sign_and_send_instruction(
            &system_instruction::create_account(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                program_pubkey.clone(),
            ),
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let txids = deploy_program_txs(program_keypair, "program/target/deploy/oracleprogram.so")
            .expect("failed to deploy program");

        println!("{:?}", txids);

        let elf = fs::read("program/target/deploy/oracleprogram.so")
            .expect("elf path should be available");
        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey.clone())
                .unwrap()
                .data[LoaderState::program_data_offset()..]
                == elf
        );

        let (txid, instruction_hash) = sign_and_send_instruction(
            Instruction {
                program_id: Pubkey::system_program(),
                accounts: vec![AccountMeta {
                    pubkey: program_pubkey.clone(),
                    is_signer: true,
                    is_writable: true,
                }],
                data: vec![2],
            },
            vec![program_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        assert!(
            read_account_info(NODE1_ADDRESS, program_pubkey.clone())
                .unwrap()
                .is_executable
        );

        let (txid, vout) = send_utxo(caller_pubkey.clone());
        println!("{}:{} {:?}", txid, vout, hex::encode(caller_pubkey));

        let (txid, instruction_hash) = sign_and_send_instruction(
            &system_instruction::create_account(
                hex::decode(txid).unwrap().try_into().unwrap(),
                vout,
                caller_pubkey.clone(),
            ),
            vec![caller_keypair],
        )
        .expect("signing and sending a transaction should not fail");

        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        println!("processed_tx {:?}", processed_tx);

        let mut old_feerate = 0;

        loop {
            let body: Value = reqwest::blocking::get(&arch_sdk::constants::get_api_endpoint_url(
                BITCOIN_NETWORK,
                "fees/recommended",
            ))
            .unwrap()
            .json()
            .unwrap();
            let feerate = body.get("fastestFee").unwrap().as_u64().unwrap();

            if old_feerate != feerate {
                let (txid, instruction_hash) = sign_and_send_instruction(
                    Instruction {
                        program_id: program_pubkey.clone(),
                        accounts: vec![AccountMeta {
                            pubkey: caller_pubkey.clone(),
                            is_signer: true,
                            is_writable: true,
                        }],
                        data: feerate.to_le_bytes().to_vec(),
                    },
                    vec![caller_keypair],
                )
                .expect("signing and sending a transaction should not fail");

                let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
                    .expect("get processed transaction should not fail");
                println!("processed_tx {:?}", processed_tx);

                println!(
                    "{:?}",
                    read_account_info(NODE1_ADDRESS, caller_pubkey.clone())
                );

                old_feerate = feerate;
            }
        }
    }
}
