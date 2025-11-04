#![cfg(test)]
pub const ELF_PATH: &str = "./program/target/sbpf-solana-solana/release/counter_program.so";

const PROGRAM_FILE_PATH: &str = ".counter_program.json";
const AUTHORITY_FILE_PATH: &str = ".counter_authority.json";

const MINING_ADDRESS: &str = "bcrt1q9s6pf9hswah20jjnzmyvk9s2xwp7srz6m2r5tw";

pub mod counter_helpers;
pub mod counter_instructions;
#[cfg(test)]
pub mod errors_and_panics;
#[cfg(test)]
pub mod happy_path;
#[cfg(test)]
pub mod intra_block_rollback_tests;
#[cfg(test)]
pub mod pruning;
#[cfg(test)]
pub mod rollback_tests;
