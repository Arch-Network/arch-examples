use arch_program::compute_budget;
use arch_program::sanitized::ArchMessage;
use arch_program::{
    account::AccountMeta, instruction::Instruction, pubkey::Pubkey, utxo::UtxoMeta,
};
use arch_sdk::blocking::{prepare_fees, ArchRpcClient, BitcoinHelper};
use arch_sdk::{build_and_sign_transaction, with_secret_key_file, Config};
use borsh::BorshSerialize;

#[derive(BorshSerialize)]
pub struct CreateAccountParams {
    /// Identifier for the new account
    pub name: String,
    /// A UTXO owned by the payer's account address; the program spends it as input 0
    /// of the Bitcoin transaction it emits
    pub utxo: UtxoMeta,
    /// Bitcoin transaction providing the fee input
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
pub fn create_new_account(program_id: Pubkey, name: String) -> std::io::Result<()> {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);
    let bitcoin_helper = BitcoinHelper::new(&config).expect("Failed to create BitcoinHelper");

    let (account_keypair, account_pubkey) = with_secret_key_file(".test_account.json")?;
    let (payer_keypair, payer_pubkey) = with_secret_key_file(".test_account.json")?;
    client
        .create_and_fund_account_with_faucet(&payer_keypair)
        .unwrap();

    // Step 1: Send a UTXO to the payer's account address.
    // The program spends this UTXO as input 0 of the Bitcoin transaction it emits.
    let (txid, vout) = bitcoin_helper.send_utxo(payer_pubkey).unwrap();
    println!(
        "UTXO created - txid: {}, vout: {}, pubkey: {}",
        txid,
        vout,
        hex::encode(payer_pubkey.serialize())
    );

    // Step 2: Retrieve a Bitcoin transaction that will be used for fee calculation
    // This ensures the transaction has appropriate fees for processing
    let tx_hex = hex::decode(
        prepare_fees().map_err(|e| std::io::Error::other(format!("prepare_fees failed: {}", e)))?,
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("hex decode error: {}", e),
        )
    })?;

    // Step 3: Package all the parameters needed for account creation
    // The UTXO is the payer-owned output the program spends
    let params = CreateAccountParams {
        name,
        utxo: UtxoMeta::from(
            hex::decode(txid)
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("hex decode error: {}", e),
                    )
                })?
                .try_into()
                .unwrap(),
            vout,
        ),
        tx_hex,
    };

    // Step 4: Create the instruction that will be sent to the program
    // This instruction contains all the necessary information for account creation
    let instruction = Instruction {
        program_id, // The program that will process this instruction
        accounts: vec![
            AccountMeta {
                pubkey: account_pubkey, // The account being created
                is_signer: true,        // This account must sign the transaction
                is_writable: true,      // The account's data will be modified
            },
            AccountMeta::new(payer_pubkey, true),
        ],
        data: borsh::to_vec(&params).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("borsh serialize error: {}", e),
            )
        })?, // Serialize the parameters into bytes
    };

    // Step 5: Sign and send the instruction to the network
    // The account_pubkey is included in the signers list as it needs to authorize this action
    let transaction = build_and_sign_transaction(
        ArchMessage::new(
            &[
                compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                instruction,
            ],
            Some(payer_pubkey),
            client.get_best_finalized_block_hash().unwrap(),
        ),
        vec![account_keypair, payer_keypair],
        config.network,
    )
    .expect("Failed to build and sign transaction");

    let txid = client.send_transaction(transaction).unwrap();
    let block_transactions = client.wait_for_processed_transaction(&txid).unwrap();

    // Step 6: Confirm successful account creation
    println!(
        "Account created successfully with transaction: {:?}",
        block_transactions
    );
    Ok(())
}
