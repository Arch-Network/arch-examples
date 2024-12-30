use anyhow::Result;
use arch_program::pubkey::Pubkey;
use arch_sdk::helper::with_secret_key_file;
use create_new_account::create_new_account;

fn main() -> Result<()> {
    // Generate or load a keypair for testing
    let (_, account_pubkey) = with_secret_key_file(".test_account.json")?;

    let program_id = Pubkey::new_unique(); // Or use your specific program ID
    create_new_account(program_id, account_pubkey, "test".to_string())?;
    Ok(())
}
