# Govrix Scout Proxy — Multi-stage Rust build
# ──────────────────────────────────────────────────────────────────────────────
# Stage 1: Build
# Stage 2: Runtime (minimal debian)
# ──────────────────────────────────────────────────────────────────────────────

# ── Stage 1: Build ────────────────────────────────────────────────────────────
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY crates/govrix-scout-common/Cargo.toml   crates/govrix-scout-common/
COPY crates/govrix-scout-store/Cargo.toml    crates/govrix-scout-store/
COPY crates/govrix-scout-proxy/Cargo.toml    crates/govrix-scout-proxy/
COPY crates/govrix-scout-cli/Cargo.toml      crates/govrix-scout-cli/
COPY crates/govrix-scout-reports/Cargo.toml  crates/govrix-scout-reports/
COPY crates/govrix-common/Cargo.toml         crates/govrix-common/
COPY crates/govrix-identity/Cargo.toml       crates/govrix-identity/
COPY crates/govrix-policy/Cargo.toml         crates/govrix-policy/
COPY crates/govrix-server/Cargo.toml         crates/govrix-server/

# Stub source files so dependency layer is cached independently of real source
RUN mkdir -p crates/govrix-scout-common/src \
    && echo 'pub fn stub() {}' > crates/govrix-scout-common/src/lib.rs \
    && mkdir -p crates/govrix-scout-store/src \
    && echo 'pub fn stub() {}' > crates/govrix-scout-store/src/lib.rs \
    && mkdir -p crates/govrix-scout-proxy/src \
    && echo 'fn main() {}' > crates/govrix-scout-proxy/src/main.rs \
    && mkdir -p crates/govrix-scout-cli/src \
    && echo 'fn main() {}' > crates/govrix-scout-cli/src/main.rs \
    && mkdir -p crates/govrix-scout-reports/src \
    && echo 'pub fn stub() {}' > crates/govrix-scout-reports/src/lib.rs \
    && mkdir -p crates/govrix-common/src \
    && echo 'pub fn stub() {}' > crates/govrix-common/src/lib.rs \
    && mkdir -p crates/govrix-identity/src \
    && echo 'pub fn stub() {}' > crates/govrix-identity/src/lib.rs \
    && mkdir -p crates/govrix-policy/src \
    && echo 'pub fn stub() {}' > crates/govrix-policy/src/lib.rs \
    && mkdir -p crates/govrix-server/src \
    && echo 'fn main() {}' > crates/govrix-server/src/main.rs

# Pre-fetch and cache all crates.io dependencies
RUN cargo fetch --locked

# Build dependencies only (cached layer)
RUN cargo build --release --frozen -p govrix-scout-proxy 2>/dev/null || true

# Copy real source and rebuild
# touch ALL source roots so cargo sees them as newer than the stub-compiled
# .rlib artifacts and recompiles every workspace crate from real source.
COPY crates/ crates/
RUN find crates -name "*.rs" -exec touch {} + && \
    cargo build --release --frozen -p govrix-scout-proxy

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/govrix-scout /usr/local/bin/govrix-scout

COPY config/ /app/config/

RUN useradd -r -s /bin/false govrix
USER govrix

EXPOSE 4000 4001 9090

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -f http://127.0.0.1:4001/health || exit 1

ENTRYPOINT ["govrix-scout"]
