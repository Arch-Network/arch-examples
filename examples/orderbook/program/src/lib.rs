use arch_program::{account::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
use arch_program::entrypoint;
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

#[allow(dead_code)]
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = processor::Processor::process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        println!("Error: {:?}", error);
        return Err(error);
    }
    Ok(())
}
