#![deny(missing_docs)]

//! A simple program that accepts a string of encoded characters and verifies that it parses. Currently handles UTF-8.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

solana_sdk::declare_id!("memobrpr6QZ6Xg59ForypkU3BsFhWEC7wHeNpBYtamC");
