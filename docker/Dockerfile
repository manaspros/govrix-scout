# AgentMesh Proxy — Multi-stage Rust build
# ──────────────────────────────────────────────────────────────────────────────
# Stage 1: Build
# Stage 2: Runtime (minimal debian)
# ──────────────────────────────────────────────────────────────────────────────

# ── Stage 1: Build ────────────────────────────────────────────────────────────
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY crates/agentmesh-common/Cargo.toml crates/agentmesh-common/
COPY crates/agentmesh-store/Cargo.toml crates/agentmesh-store/
COPY crates/agentmesh-proxy/Cargo.toml crates/agentmesh-proxy/
COPY crates/agentmesh-cli/Cargo.toml crates/agentmesh-cli/
COPY crates/agentmesh-reports/Cargo.toml crates/agentmesh-reports/

# Create stub lib.rs / main.rs files to cache dependencies
RUN mkdir -p crates/agentmesh-common/src \
    && echo 'pub fn lib_stub() {}' > crates/agentmesh-common/src/lib.rs \
    && mkdir -p crates/agentmesh-store/src \
    && echo 'pub fn lib_stub() {}' > crates/agentmesh-store/src/lib.rs \
    && mkdir -p crates/agentmesh-proxy/src \
    && echo 'fn main() {}' > crates/agentmesh-proxy/src/main.rs \
    && mkdir -p crates/agentmesh-cli/src \
    && echo 'fn main() {}' > crates/agentmesh-cli/src/main.rs \
    && mkdir -p crates/agentmesh-reports/src \
    && echo 'pub fn lib_stub() {}' > crates/agentmesh-reports/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release -p agentmesh-proxy 2>/dev/null; true

# Now copy the real source
COPY crates/ crates/

# Build the real binary
RUN cargo build --release -p agentmesh-proxy

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/agentmesh /usr/local/bin/agentmesh

# Copy default config
COPY config/agentmesh.default.toml /app/config/agentmesh.default.toml

# Create non-root user
RUN useradd -r -s /bin/false agentmesh
USER agentmesh

# Expose ports
EXPOSE 4000 4001 9090

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:4001/health || exit 1

ENTRYPOINT ["agentmesh"]
