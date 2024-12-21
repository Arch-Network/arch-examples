use crate::counter_helpers::print_title;

pub fn try_deploy_program(
    elf_path: &str,
    program_file_path: &str,
    program_name: &str,
) -> anyhow::Result<arch_program::pubkey::Pubkey> {
    use arch_program::system_instruction::SystemInstruction;
    use arch_sdk::constants::*;
    use arch_sdk::helper::*;
    use std::fs;
    use tracing::{debug, error};

    print_title(&format!("PROGRAM DEPLOYMENT {}", program_name), 5);

    let (program_keypair, program_pubkey) =
        with_secret_key_file(program_file_path).expect("getting caller info should not fail");

    let elf = fs::read(elf_path).expect("elf path should be available");

    if let Ok(account_info_result) = read_account_info(NODE1_ADDRESS, program_pubkey) {
        if account_info_result.data != elf {
            error!("Program account content is different from provided ELF file !");
            panic!();
        }
        println!("\x1b[33m Same program already deployed ! Skipping deployment. \x1b[0m");
        return Ok(program_pubkey);
    };

    let (deploy_utxo_btc_txid, deploy_utxo_vout) = send_utxo(program_pubkey);

    println!(
        "\x1b[32m Step 1/4 Successful :\x1b[0m BTC Transaction for program account UTXO successfully sent : https://mempool.dev.aws.archnetwork.xyz/tx/{} -- vout : {}",
        deploy_utxo_btc_txid, deploy_utxo_vout
    );

    let (pa_arch_txid, _pa_arch_txid_hash) = sign_and_send_instruction(
        SystemInstruction::new_create_account_instruction(
            hex::decode(deploy_utxo_btc_txid)
                .unwrap()
                .try_into()
                .unwrap(),
            deploy_utxo_vout,
            program_pubkey,
        ),
        vec![program_keypair],
    )
    .expect("signing and sending a transaction should not fail");

    let _processed_tx = get_processed_transaction(NODE1_ADDRESS, pa_arch_txid.clone())
        .expect("get processed transaction should not fail");

    println!("\x1b[32m Step 2/4 Successful :\x1b[0m Program account creation transaction successfully processed !.\x1b[0m");

    debug!("{:?}", _processed_tx);

    deploy_program_txs(program_keypair, elf_path)?;

    let elf = fs::read(elf_path).expect("elf path should be available");

    let program_info_after_deployment = read_account_info(NODE1_ADDRESS, program_pubkey).unwrap();

    assert!(program_info_after_deployment.data == elf);

    debug!(
        "Current Program Account {:x}: \n   Owner : {}, \n   Data length : {} Bytes,\n   Anchoring UTXO : {}, \n   Executable? : {}",
        program_pubkey, program_info_after_deployment.owner,
        program_info_after_deployment.data.len(),
        program_info_after_deployment.utxo,
        program_info_after_deployment.is_executable
    );

    println!("\x1b[32m Step 3/4 Successful :\x1b[0m Sent ELF file as transactions, and verified program account's content against local ELF file!");

    let (executability_txid, _) = sign_and_send_instruction(
        SystemInstruction::new_deploy_instruction(program_pubkey),
        vec![program_keypair],
    )
    .expect("signing and sending a transaction should not fail");

    let _processed_tx = get_processed_transaction(NODE1_ADDRESS, executability_txid.clone())
        .expect("get processed transaction should not fail");

    let program_info_after_making_executable =
        read_account_info(NODE1_ADDRESS, program_pubkey).unwrap();

    debug!(
        "Current Program Account {:x}: \n   Owner : {:x}, \n   Data length : {} Bytes,\n   Anchoring UTXO : {}, \n   Executable? : {}",
        program_pubkey,
        program_info_after_making_executable.owner,
        program_info_after_making_executable.data.len(),
        program_info_after_making_executable.utxo,
        program_info_after_making_executable.is_executable
    );

    assert!(program_info_after_making_executable.is_executable);

    println!("\x1b[32m Step 4/4 Successful :\x1b[0m Made program account executable!");

    print_title("PROGRAM DEPLOYMENT : OK !", 5);

    Ok(program_pubkey)
}
