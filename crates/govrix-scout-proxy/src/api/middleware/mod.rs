//! Middleware modules for the management API.
//!
//! - `cors` — CORS configuration (permissive dev, restricted prod)
//! - `auth` — Optional bearer token authentication

pub mod auth;
pub mod cors;
