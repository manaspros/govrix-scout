#!/usr/bin/env bash
set -euo pipefail

echo "Setting up Govrix Platform..."

# Check prerequisites
command -v cargo >/dev/null 2>&1 || { echo "Rust/cargo not found. Install from https://rustup.rs"; exit 1; }
command -v psql >/dev/null 2>&1 || { echo "PostgreSQL not found. Install postgres 15+"; exit 1; }

# Build all crates
echo "Building..."
cargo build --release

echo "Done. Run with: GOVRIX_LICENSE_KEY=<key> ./target/release/govrix-server"
