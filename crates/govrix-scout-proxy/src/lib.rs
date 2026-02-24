//! Govrix Scout proxy library crate.
//!
//! Exposes the proxy, API, events, and policy modules so that the
//! govrix-scout-proxy binary and the Govrix Platform can both depend on
//! this crate as a library.
//!
//! # Binary vs library
//!
//! The binary entry point lives in `src/main.rs`. It imports everything
//! via `use govrix_scout_proxy::*` instead of declaring its own `mod` blocks,
//! so there is a single canonical source for each module.

pub mod api;
pub mod events;
pub mod policy;
pub mod proxy;
