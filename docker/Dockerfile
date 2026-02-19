FROM rust:1.85-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y openssh-client git pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# SSH setup for private git dependencies
ARG SSH_PRIVATE_KEY
RUN mkdir -p /root/.ssh && \
    echo "$SSH_PRIVATE_KEY" > /root/.ssh/id_ed25519 && \
    chmod 600 /root/.ssh/id_ed25519 && \
    ssh-keyscan github.com >> /root/.ssh/known_hosts && \
    git config --global url."ssh://git@github.com/".insteadOf "https://github.com/"

# Use git CLI for fetching (more reliable SSH agent forwarding)
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY config/ config/

RUN cargo build --release -p govrix-server && \
    rm -rf /root/.ssh

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/govrix /usr/local/bin/govrix
COPY --from=builder /app/config/ /etc/govrix/

ENV GOVRIX_CONFIG=/etc/govrix/govrix.default.toml
EXPOSE 4000 4001
CMD ["govrix"]
