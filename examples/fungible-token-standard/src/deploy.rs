use anyhow::anyhow;
use arch_program::{
    account::AccountMeta, instruction::Instruction, system_instruction::SystemInstruction,
};
use arch_sdk::helper::{
    build_and_send_block, build_transaction, fetch_processed_transactions, generate_new_keypair,
    init_logging,
};
use arch_sdk::processed_transaction::Status;
use arch_sdk::{
    constants::{NODE1_ADDRESS, PROGRAM_FILE_PATH},
    helper::{get_processed_transaction, read_account_info, send_utxo, sign_and_send_instruction},
};
use ebpf_counter::counter_deployment::try_deploy_program;
use fungible_token_standard_program::mint::InitializeMintInput;
use serial_test::serial;

use borsh::BorshSerialize;

use crate::{
    helpers::{
        create_balance_account, get_balance_account, get_mint_info,
        provide_empty_account_to_program,
    },
    standard_tests::ELF_PATH,
};

#[ignore]
#[serial]
#[test]
fn deploy_standard_program() {
    init_logging();

    let program_pubkey =
        try_deploy_program(ELF_PATH, PROGRAM_FILE_PATH, "Fungible-Token-Standard").unwrap();

    println!(
        "Deployed Fungible token standard program account id {:?}!",
        program_pubkey.serialize()
    );

    let (mint_account_keypair, mint_account_pubkey) =
        provide_empty_account_to_program(&program_pubkey).unwrap();

    let mint_input = InitializeMintInput::new(
        mint_account_pubkey.serialize(),
        1000000,
        "SPONK".to_string(),
        1,
    );

    let mut instruction_data = vec![0u8];

    mint_input
        .serialize(&mut instruction_data)
        .expect("Couldnt serialize mint input");

    let initialize_mint_instruction = Instruction {
        program_id: program_pubkey,
        accounts: vec![AccountMeta {
            pubkey: mint_account_pubkey,
            is_signer: true,
            is_writable: true,
        }],
        data: instruction_data,
    };

    let transaction = build_transaction(
        vec![mint_account_keypair],
        vec![initialize_mint_instruction],
    );

    let block_transactions = build_and_send_block(vec![transaction]);

    let processed_transactions = fetch_processed_transactions(block_transactions).unwrap();

    assert!(matches!(
        processed_transactions[0].status,
        Status::Processed
    ));

    let mint_details = get_mint_info(&mint_account_pubkey).expect("Couldnt deserialize mint info");

    println!("Transaction logs : /home/spaceman/ArchNetwork/main-arch-network/arch-network/.arch-data/arch-validator-data-0/trace/{}.txt", processed_transactions[0].txid());

    println!("Mint account  {:?}", mint_account_pubkey.serialize());

    let (account_keypair, account_pubkey, address) = generate_new_keypair();

    let (txid, vout) = send_utxo(account_pubkey);

    let (txid, _) = sign_and_send_instruction(
        SystemInstruction::new_create_account_instruction(
            hex::decode(txid).unwrap().try_into().unwrap(),
            vout,
            account_pubkey,
        ),
        vec![account_keypair],
    )
    .expect("signing and sending a transaction should not fail");

    let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
        .expect("get processed transaction should not fail");

    assert!(matches!(processed_tx.status, Status::Processed));

    println!("User account {:?}", account_pubkey.serialize());

    let account_info = read_account_info(NODE1_ADDRESS, account_pubkey)
        .map_err(|e| anyhow!(format!("Error reading account content {}", e.to_string())))
        .unwrap();

    let account_info = account_info;

    println!("Created user account ! {:?}", account_info);

    let balance_account_pubkey = create_balance_account(
        &account_pubkey,
        account_keypair,
        &mint_account_pubkey,
        &program_pubkey,
    )
    .unwrap();

    let balance_account = get_balance_account(&balance_account_pubkey).unwrap();

    println!("Balance account {:?}", balance_account);
}
