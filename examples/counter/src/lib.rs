#![cfg(test)]
pub const ELF_PATH: &str = "./program/target/sbf-solana-solana/release/counter_program.so";

pub mod counter_helpers;
pub mod counter_instructions;
#[cfg(test)]
pub mod errors_and_panics;
#[cfg(test)]
pub mod happy_path;
#[cfg(test)]
pub mod intra_block_rollback_tests;
#[cfg(test)]
pub mod rollback_tests;
