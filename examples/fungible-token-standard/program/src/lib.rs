/* ---------- BUILD USING CARGO_TARGET_DIR=./target cargo build-sbf --------- */
use arch_program::{
    account::AccountInfo, entrypoint, msg, program::next_account_info, program_error::ProgramError,
    pubkey::Pubkey,
};
use mint::{initialize_mint, mint_tokens, InitializeMintInput, MintInput};
use token_account::initialize_balance_account;
use transfer::{transfer_tokens, TransferInput};
pub mod errors;
pub mod mint;
pub mod token_account;
pub mod transfer;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    match instruction_data[0] {
        0 => {
            /* -------------------------------------------------------------------------- */
            /*                               INITIALIZE MINT                              */
            /* -------------------------------------------------------------------------- */
            // 1 Account : (owned by program, uninitialized)
            msg!("Initializing Mint Account ");

            if accounts.len() != 1 {
                return Err(ProgramError::Custom(502));
            }

            let account = next_account_info(account_iter)?;

            let initialize_mint_input: InitializeMintInput =
                borsh::from_slice(&instruction_data[1..])
                    .map_err(|_e| ProgramError::InvalidArgument)?;

            initialize_mint(account, program_id, initialize_mint_input)?;
        }
        1 => {
            /* -------------------------------------------------------------------------- */
            /*                         INITIALIZE BALANCE ACCOUNT                         */
            /* -------------------------------------------------------------------------- */
            // No instruction data needed, only 3 accounts
            // 1 - Token balance owner (signer, writable)
            // 2 - Mint account (owned by program and writable)
            // 3 - Supplied account (owned by program, uninitialized )
            if accounts.len() != 3 {
                return Err(ProgramError::Custom(502));
            }

            let owner_account = next_account_info(account_iter)?;

            let mint_account = next_account_info(account_iter)?;

            let balance_account = next_account_info(account_iter)?;

            initialize_balance_account(owner_account, mint_account, balance_account, program_id)?;
        }
        2 => {
            /* -------------------------------------------------------------------------- */
            /*                                 MINT TOKENS                                */
            /* -------------------------------------------------------------------------- */
            // 1 - Mint account ( owned by program and writable )
            // 2 - Balance account ( owned by program and writable )
            // 3 - Owner account( signer )
            if accounts.len() != 3 {
                return Err(ProgramError::Custom(502));
            }

            let mint_account = next_account_info(account_iter)?;

            let balance_account = next_account_info(account_iter)?;

            let owner_account = next_account_info(account_iter)?;

            let mint_input: MintInput = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;

            mint_tokens(
                balance_account,
                mint_account,
                owner_account,
                program_id,
                mint_input,
            )?;
        }
        3 => {
            /* -------------------------------------------------------------------------- */
            /*                               TRANSFER TOKENS                              */
            /* -------------------------------------------------------------------------- */
            // 1 - Owner Account ( is_signer )
            // 2 - Mint Account ( writable and owned by program )
            // 3 - Sender Account ( writable and owned by program, balance owner is Account 1 )
            // 4 - Receiver Account ( writable and owned by program )

            if accounts.len() != 4 {
                return Err(ProgramError::Custom(502));
            }

            let owner_account = next_account_info(account_iter)?;

            let mint_account = next_account_info(account_iter)?;

            let sender_account = next_account_info(account_iter)?;

            let receiver_account = next_account_info(account_iter)?;

            let transfer_input: TransferInput = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;

            transfer_tokens(
                owner_account,
                mint_account,
                sender_account,
                receiver_account,
                program_id,
                transfer_input,
            )?;
        }
        // 4 => {
        //     /* -------------------------------------------------------------------------- */
        //     /*                                 BURN TOKENS                                */
        //     /* -------------------------------------------------------------------------- */
        //     // 1 - Owner Account ( is_signer )
        //     // 2 - Mint Account ( writable and owned by program )
        //     // 3 - Burner Account ( writable and owned by program, balance owner is Account 1 )
        //     // 4 - Token Burn Account ( writable and owned by program )

        //     if accounts.len() != 4 {
        //         return Err(ProgramError::Custom(502));
        //     }

        //     let owner_account = next_account_info(account_iter)?;

        //     let mint_account = next_account_info(account_iter)?;

        //     let sender_account = next_account_info(account_iter)?;

        //     let token_burn_account = next_account_info(account_iter)?;

        //     let transfer_input: TransferInput = borsh::from_slice(&instruction_data[1..])
        //         .map_err(|_e| ProgramError::InvalidArgument)?;

        //     transfer_tokens(
        //         owner_account,
        //         mint_account,
        //         sender_account,
        //         token_burn_account,
        //         program_id,
        //         transfer_input,
        //         true,
        //     )?;
        // }
        _ => {
            msg!("Invalid argument provided !");
            return Err(ProgramError::InvalidArgument);
        }
    }

    Ok(())
}
