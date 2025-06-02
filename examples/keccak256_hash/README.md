# Keccak256 Hash Example

This example demonstrates the Keccak256 (SHA-3) hashing functionality available in Arch Network programs through the `arch_program::keccak::hash` syscall.

## Overview

The Keccak256 hash function is a cryptographic hash function that produces a 256-bit (32-byte) hash digest. This implementation provides access to the same hashing algorithm used by Ethereum and other blockchain systems.

## Project Structure

```
examples/keccak256_hash/
├── README.md                    # This file
├── Cargo.toml                   # Client dependencies and configuration
├── .program.json                # Program deployment configuration
├── src/
│   └── lib.rs                   # Integration tests and examples
└── program/
    ├── Cargo.toml               # Program dependencies
    └── src/
        └── lib.rs               # On-chain program implementation
```

## Features

### Test Types Supported
- **Single Input**: Hash a single byte array
- **Multiple Inputs**: Hash multiple inputs separately and combine their results
- **Empty Input**: Hash empty data (known test vector)
- **Large Input**: Performance testing with larger datasets
- **Known Vectors**: Validation against standard Keccak256 test vectors

### Known Test Vectors Implemented
- Empty string: `c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470`
- "abc": `4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45`
- "The quick brown fox jumps over the lazy dog": `4d741b6f1eb29cb2a9b9911c82f56fa8d73b04959d3d9d222895df6c0b28aa15`

## Building

### Build the Program
```bash
cd program
cargo build-sbf
```

### Build the Client
```bash
cargo build
```

## Testing

### Unit Tests (Reference Implementation)
Run standalone tests that validate our expected hash values using the reference implementation:

```bash
cargo test --lib standalone_tests
```

### Integration Tests (Requires Runtime)
**Note**: Integration tests currently fail due to a workspace-wide dependency issue (`_sol_set_return_data` symbol missing). This is not related to the Keccak256 implementation but affects all examples in the workspace.

```bash
# These tests require a running Arch Network node
cargo test --lib keccak256_hash_tests -- --ignored
```

## Implementation Details

### Program Side (`program/src/lib.rs`)
The on-chain program implements five main test functions:
- `test_single_input_hash()`: Direct hashing of input data
- `test_multiple_inputs_hash()`: Combines hashes of separate inputs
- `test_empty_input_hash()`: Validates empty input against known vector
- `test_large_input_hash()`: Creates larger dataset and hashes it
- `test_known_vectors()`: Tests against standard test vectors

### Client Side (`src/lib.rs`)
The client provides:
- Comprehensive integration tests
- Reference implementation validation using `sha3` crate
- Test data generation and verification
- Performance benchmarking capabilities

### Data Structures
```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Keccak256HashParams {
    pub test_type: TestType,
    pub data: Vec<u8>,
    pub additional_data: Option<Vec<u8>>,
    pub tx_hex: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum TestType {
    SingleInput,
    MultipleInputs,
    EmptyInput,
    LargeInput,
    KnownVectors,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Keccak256Result {
    pub hash: [u8; 32],
    pub test_type: TestType,
    pub input_size: u32,
    pub success: bool,
}
```

## Usage Example

```rust
use arch_program::keccak::hash as keccak256;

// Hash some data
let data = b"Hello, Arch Network!";
let hash = keccak256(data);
println!("Hash: {:?}", hash.0);
```

## Dependencies

### Program Dependencies
- `arch_program`: Core program functionality and syscalls
- `borsh`: Serialization/deserialization

### Client Dependencies
- `arch_sdk`: Arch Network SDK for transactions and RPC
- `arch_test_sdk`: Testing utilities
- `sha3`: Reference implementation for validation
- `hex`: Hexadecimal encoding/decoding
- `serial_test`: Sequential test execution

## Troubleshooting

### Known Issues
1. **Integration test failures**: Due to missing `_sol_set_return_data` symbol in workspace dependencies. This affects all examples, not just Keccak256.
2. **Program compilation**: Requires compatible Rust toolchain version (1.79.0-dev for Solana BPF).

### Solutions
- Use standalone unit tests for validation
- Ensure program builds successfully with `cargo build-sbf`
- Reference implementation tests work correctly

## Performance Characteristics

The Keccak256 implementation provides:
- Consistent hash output for identical inputs
- Support for variable-length input data
- Efficient processing of both small and large datasets
- Standards-compliant output matching Ethereum's Keccak256

## Security Considerations

- Uses the same Keccak256 algorithm as Ethereum
- Cryptographically secure hash function
- Suitable for blockchain applications
- Deterministic output for identical inputs

## Future Enhancements

Potential improvements for this example:
1. **Batch Processing**: Support for hashing multiple independent inputs in a single transaction
2. **Streaming Interface**: Support for hashing large data that doesn't fit in a single transaction
3. **Performance Benchmarks**: Detailed timing analysis for different input sizes
4. **Integration Examples**: Real-world use cases like merkle tree construction 