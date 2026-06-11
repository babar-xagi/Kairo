//! Rust native bridge foundation for Kairo.
//!
//! Phase 5 will add `src/native.rs` detection, Rust shared library builds,
//! and Kotlin bridge generation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBridgeStatus {
    Planned,
}
