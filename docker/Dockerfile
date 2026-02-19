FROM rust:1.82-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y openssh-client git && rm -rf /var/lib/apt/lists/*
ARG SSH_PRIVATE_KEY
RUN mkdir -p /root/.ssh && \
    echo "$SSH_PRIVATE_KEY" > /root/.ssh/id_ed25519 && \
    chmod 600 /root/.ssh/id_ed25519 && \
    ssh-keyscan github.com >> /root/.ssh/known_hosts

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release -p govrix-server && \
    rm -rf /root/.ssh

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/govrix /usr/local/bin/govrix
EXPOSE 8080 4001
CMD ["govrix"]
