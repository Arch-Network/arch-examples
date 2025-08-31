#![cfg(test)]
use std::{fs, path::PathBuf};

use arch_program::{
    bitcoin::{
        key::{Keypair, Secp256k1},
        secp256k1::SecretKey,
    },
    pubkey::Pubkey,
};
use arch_sdk::{ArchRpcClient, Config};
use hex::decode;

pub(crate) fn get_peer_keypair_from_file(peer_number: u8) -> Keypair {
    let path = format!(
        "../../.arch-data/arch-validator-data-{}/localnet/identity-secret",
        peer_number
    );

    let path_buffer = PathBuf::from(path);
    let path = path_buffer.canonicalize().unwrap();
    let file_content = fs::read(path).unwrap();
    let hex_str = String::from_utf8(file_content).unwrap();
    let secret_key_bytes = decode(hex_str).unwrap();

    let secret_key = SecretKey::from_slice(&secret_key_bytes).unwrap();
    let secp = Secp256k1::new();

    Keypair::from_secret_key(&secp, &secret_key)
}

pub(crate) fn get_bootnode_keypair_from_file() -> Keypair {
    let path = "../../.arch-bootnode-data/localnet/identity-secret".to_string();
    let path_buffer = PathBuf::from(path);
    let path = path_buffer.canonicalize().unwrap();
    let file_content = fs::read(path).unwrap();
    let hex_str = String::from_utf8(file_content).unwrap();
    let secret_key_bytes = decode(hex_str).unwrap();

    let secret_key = SecretKey::from_slice(&secret_key_bytes).unwrap();
    let secp = Secp256k1::new();

    Keypair::from_secret_key(&secp, &secret_key)
}

pub fn try_to_create_and_fund_account(keypair: &Keypair) {
    let config = Config::localnet();
    let client = ArchRpcClient::new(&config);
    let keypair_pubkey = keypair.public_key().x_only_public_key().0.serialize();
    let keypair_arch_pubkey = Pubkey::from_slice(&keypair_pubkey);
    let account_info = client.read_account_info(keypair_arch_pubkey);

    if account_info.is_ok() {
        println!("\x1b[33m\x1b[1mAccount already exists, skipping creation ! \x1b[0m");
    } else {
        println!("Account does not exist, creating it !");
        client.create_and_fund_account_with_faucet(keypair).unwrap();
        println!("Account created and funded !");
    }
}

#[cfg(test)]
mod utils_tests {

    use super::*;

    #[ignore]
    #[test]
    fn test_get_bootnode_keypair_from_file() {
        let bootnode_keypair = get_bootnode_keypair_from_file();
        println!(
            "Successfully retrieved Bootnode keypair with pubkey {:?}",
            bootnode_keypair.public_key()
        );
    }

    #[ignore]
    #[test]
    fn test_keypair_fetch_for_peer() {
        let _peer_keypair = get_peer_keypair_from_file(1);
    }
}
