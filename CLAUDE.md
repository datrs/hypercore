# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the **hypercore** crate, a Rust implementation of the Hypercore protocol - a secure, distributed append-only log. It aims for binary compatibility with the Javascript LTS version for disk storage while maintaining interoperability with the wire protocol.

Key characteristics:
- **100% Safe Rust**: Uses `#![forbid(unsafe_code)]`
- **Structured concurrency**: Avoids `tokio::spawn`; work only happens when polled

## JavaScript implementation

The original JavaScript implementation of hypercore is located in  `/home/blake/git/hyper/js/hypercore/.`.

## Building and Testing

### Basic commands
```bash
# Run tests
cargo test

# Run linting (note: strict lints enabled)
cargo clippy
```

### Running specific tests
```bash
# Run a single test
cargo test test_name

# Run tests with specific runtime
cargo test --no-default-features --features tokio
cargo test --no-default-features --features async-std
```

### JavaScript interoperability tests
To verify compatibility with the original Javascript implementation:
```bash
cargo test --features js_interop_tests
```

These tests use the `rusty_nodejs_repl` crate to run Javascript code side-by-side with Rust, verifying that both implementations produce identical on-disk results.

### Running examples
```bash
cargo run --example memory
cargo run --example disk
cargo run --example replication
```

## Architecture

## Style guide

Favor reduced nesting wherever  possible.

### Core Data Structures

The `Hypercore` struct is the main entry point, built using `HypercoreBuilder`. Internally, it consists of:

- **Oplog** (`oplog/`): Append-only operation log with dual-header structure for crash recovery. Headers alternate between two memory areas for atomic updates. Stores entries with CRC checksums.

- **MerkleTree** (`tree/`): Sparse merkle tree storing cryptographic hashes. Uses root indices to track tree state. Supports optional Moka cache for node lookups (with `cache` feature).

- **BlockStore** (`data/`): Stores actual data blocks indexed by position.

- **Bitfield** (`bitfield/`): Tracks which blocks are available locally (for sparse replication). Has `fixed` and `dynamic` implementations.

- **Storage** (`storage/`): Abstraction over storage backends (memory/disk). Splits data into four stores: `tree`, `data`, `bitfield`, and `oplog`. Non-WASM targets use `random-access-disk`; WASM uses `random-access-memory`.

### Replication

The `replication/` module (enabled with `replication` feature, on by default) handles:

- **Events** (`events.rs`): Broadcast events to peers using `async-broadcast`
- **Peer**: Represents remote peers with protocol state

Replication works by creating proofs in a source hypercore and verifying/applying them to destination hypercores.

### Key Modules

- **crypto/**: Cryptographic operations (signing, verification, hashing) using ed25519-dalek and blake2
- **encoding.rs**: Compact binary encoding utilities
- **builder.rs**: Fluent API for constructing Hypercore instances
- **common/**: Shared types including error types, store definitions, and internal utilities

## Features

Default features: `sparse`, `replication`, `cache`

- `tokio` / `async-std`: Choose async runtime (mutually exclusive, at least one required)
- `sparse`: Enables sparse file support for disk storage
- `replication`: Enables replication support and event broadcasting
- `cache`: Moka-based cache for merkle tree nodes
- `js_interop_tests`: Enables Javascript interoperability tests (dev-only)

## Code Style and Lints

This crate has strict linting enabled (see `.clippy.toml` and `lib.rs`):
- `#![forbid(unsafe_code)]` - no unsafe code allowed
- `#![cfg_attr(test, deny(warnings))]` - warnings become errors in tests
- Additional lints: `unreachable_pub`, `redundant_lifetimes`, `clippy::needless_pass_by_value`, etc.

All public APIs must have documentation (`missing_docs` warning).

## Testing Strategy

Tests are organized in:
- `tests/core.rs`: Core functionality tests
- `tests/model.rs`: Property-based tests using `proptest`
- `tests/js_interop.rs`: Javascript interoperability verification
- `tests/common/`: Shared test utilities
- `tests/js/`: Javascript test harness code

## Important Implementation Details

- **Header structure**: Uses dual-header approach where headers alternate between two 4KB regions for atomic updates
- **Oplog entries**: Each entry has an 8-byte "leader" (4-byte CRC + 4-byte length/flags)
- **Storage flush**: Uses `skip_flush_count` for batching writes (mirrors JS `autoFlush`)
- **Node indexing**: Uses flat-tree indices for merkle tree navigation
- **Crash recovery**: Oplog design ensures consistency even if process crashes during write
