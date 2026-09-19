//! Public configuration surface.
//!
//! Re-exports from `schema` so consumers can simply write
//! `use hypr_phone::config::Config;`.

pub mod schema;

pub use schema::*;
