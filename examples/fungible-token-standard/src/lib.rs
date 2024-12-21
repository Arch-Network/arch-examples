#[cfg(test)]
pub mod deploy;
#[cfg(test)]
pub mod helpers;
#[cfg(test)]
pub mod instruction;
#[cfg(test)]
pub mod tests_mint;
#[cfg(test)]
pub mod tests_transfer;
#[cfg(test)]
pub mod standard_tests {

    pub const MINT_FILE_PATH: &str = ".mint.json";
    pub const MINT_OWNER_FILE_PATH: &str = ".mint_owner.json";

    pub const ELF_PATH: &str =
        "./program/target/sbf-solana-solana/release/fungible_token_standard_program.so";
}
