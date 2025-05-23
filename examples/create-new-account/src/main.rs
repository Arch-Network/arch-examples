use anyhow::Result;
use arch_program::pubkey::Pubkey;
use create_new_account::create_new_account;

fn main() -> Result<()> {
    let program_id = Pubkey::new_unique(); // Or use your specific program ID
    create_new_account(program_id, "test".to_string())?;
    Ok(())
}
