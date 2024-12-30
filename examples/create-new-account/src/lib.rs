use anyhow::Result;
use arch_program::{
    account::AccountMeta, instruction::Instruction, pubkey::Pubkey, utxo::UtxoMeta,
};
use arch_sdk::helper::{prepare_fees, send_utxo, sign_and_send_instruction};
use bitcoin::key::Keypair;
use bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use borsh::BorshSerialize;
use rand::rngs::OsRng;

#[derive(BorshSerialize)]
pub struct CreateAccountParams {
    pub name: String,
    pub utxo: UtxoMeta,
    pub tx_hex: Vec<u8>,
}

/// Creates a new account in the program with the specified parameters
///
/// # Arguments
/// * `program_id` - The public key of the program that will process this instruction
/// * `account_pubkey` - The public key of the new account being created
///
/// # Returns
/// * `Result<()>` - Success or error status of the account creation
pub fn create_new_account(program_id: Pubkey, account_pubkey: Pubkey, name: String) -> Result<()> {
    // Step 1: Create and send a UTXO (Unspent Transaction Output) to the new account
    // This UTXO will be used to fund the account creation
    let (txid, vout) = send_utxo(account_pubkey);
    println!(
        "UTXO created - txid: {}, vout: {}, pubkey: {}",
        txid,
        vout,
        hex::encode(account_pubkey.serialize())
    );

    // Step 2: Retrieve a Bitcoin transaction that will be used for fee calculation
    // This ensures the transaction has appropriate fees for processing
    let tx_hex = hex::decode(prepare_fees())?;

    // Step 3: Package all the parameters needed for account creation
    let params = CreateAccountParams {
        name,
        utxo: UtxoMeta::from(hex::decode(txid)?.try_into().unwrap(), vout),
        tx_hex,
    };

    // Step 4: Create the instruction that will be sent to the program
    // This instruction contains all the necessary information for account creation
    let instruction = Instruction {
        program_id, // The program that will process this instruction
        accounts: vec![AccountMeta {
            pubkey: account_pubkey, // The account being created
            is_signer: true,        // This account must sign the transaction
            is_writable: true,      // The account's data will be modified
        }],
        data: borsh::to_vec(&params)?, // Serialize the parameters into bytes
    };

    // Step 5: Sign and send the instruction to the network
    // The account_pubkey is included in the signers list as it needs to authorize this action
    let secp = Secp256k1::new();
    let (secret_key, _) = secp.generate_keypair(&mut OsRng);
    let account_keypair = Keypair::from_secret_key(&secp, &secret_key);
    let account_pubkey =
        Pubkey::from_slice(&XOnlyPublicKey::from_keypair(&account_keypair).0.serialize());
    let (txid, _) = sign_and_send_instruction(instruction, vec![account_keypair])?;

    // Step 6: Confirm successful account creation
    println!("Account created successfully with transaction: {}", txid);
    Ok(())
}
