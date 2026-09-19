//! Session bridge service.
//!
//! Provides cross-CLI-invocation session knowledge via XDG_RUNTIME_DIR
//! until hypr-phoned provides authoritative in-memory state.

pub mod bridge;

pub use bridge::*;
