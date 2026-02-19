# Contributing to Govrix Platform

This is a commercial product — external contributions are not currently accepted.

## Internal Development

### Prerequisites
- Rust 1.75+
- PostgreSQL 15+
- Docker (optional, for integration testing)

### Setup
```bash
cp config/govrix.default.toml config/govrix.toml
# Edit config/govrix.toml as needed
```

### Running
```bash
GOVRIX_LICENSE_KEY=<key> cargo run -p govrix-server
```

### Testing
```bash
cargo test           # all unit tests
cargo clippy -- -D warnings  # lint
```

### Generating a License Key
```bash
cargo run -p govrix-keygen -- --tier enterprise --org "Acme Corp" --max-agents 100
```

### Crate Overview
| Crate | Purpose |
|-------|---------|
| govrix-common | Config, license types, tenant registry |
| govrix-policy | Policy engine, PII masking, budget tracking |
| govrix-identity | mTLS CA and cert issuance |
| govrix-server | Main binary: proxy + management API |
| govrix-keygen | License key generation CLI |
