# Contributing to Govrix Scout OSS

Thank you for your interest in contributing. Govrix Scout is Apache 2.0 licensed and welcomes contributions of all kinds — bug reports, documentation improvements, new features, and code reviews.

## Table of Contents

- [Development Environment](#development-environment)
- [Project Structure](#project-structure)
- [Code Standards](#code-standards)
- [Conventional Commits](#conventional-commits)
- [Pull Request Process](#pull-request-process)
- [Issue Labels](#issue-labels)
- [Architecture Decision Records](#architecture-decision-records)
- [Testing Policy](#testing-policy)
- [Security](#security)

---

## Development Environment

### Requirements

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust | 1.82 | [rustup.rs](https://rustup.rs) |
| Node.js | 20.x | [nodejs.org](https://nodejs.org) |
| pnpm | 9.x | `corepack enable && corepack prepare pnpm@latest --activate` |
| Docker | 25+ | [docs.docker.com](https://docs.docker.com/get-docker/) |
| Docker Compose | v2 | Included with Docker Desktop |

### First-time setup

```bash
git clone https://github.com/Govrix Scout/Govrix Scout.git
cd Govrix Scout
./scripts/setup.sh
```

The setup script checks all dependencies, installs Rust toolchain components (`clippy`, `rustfmt`), and installs dashboard `node_modules`.

### Start a development stack

```bash
# Terminal 1 — PostgreSQL + TimescaleDB
make docker-up

# Terminal 2 — Rust proxy + REST API (port 4000 + 4001)
make dev-proxy

# Terminal 3 — Vite dashboard dev server (port 3000, hot reload)
make dev-dashboard
```

### Environment variables

Copy `.env.example` to `.env` if it exists, or set these directly:

```bash
# Required for the proxy to write events
DATABASE_URL=postgres://Govrix Scout:govrix_scout_dev@localhost:5432/Govrix Scout

# Optional — restrict API access
govrix_scout_API_KEY=dev-secret

# Rust log verbosity
RUST_LOG=Govrix Scout=debug,tower_http=info
```

---

## Project Structure

```
Govrix Scout/
├── crates/
│   ├── govrix-scout-common/       # Shared types: protocols, events, config, errors
│   ├── govrix-scout-store/        # Database layer: sqlx queries for events + agents
│   ├── govrix-scout-proxy/        # Proxy hot path (hyper) + REST API (axum) + policy engine
│   ├── govrix-scout-cli/          # CLI binary using clap
│   └── govrix-scout-reports/      # Report templates (minijinja) + PDF generation
├── dashboard/                  # React 18 + TypeScript + Vite + Recharts
├── migrations/                 # SQL migration files (applied by docker-entrypoint-initdb.d)
├── config/                     # Default TOML configuration
├── docker/                     # Dockerfiles + docker-compose.yml + nginx.conf
├── scripts/                    # setup.sh, seed-demo-data.sh
├── tests/                      # Integration tests (require live PostgreSQL)
└── docs/adr/                   # Architecture Decision Records
```

### Crate responsibilities

- **govrix-scout-common**: No I/O. Pure types, parsing, serialization, config loading.
- **govrix-scout-store**: All database access. No HTTP, no business logic.
- **govrix-scout-proxy**: Orchestrates everything. The `proxy/` module uses hyper (hot path). The `api/` module uses axum (management path). The `policy/` module is call-by-value, no async.
- **govrix-scout-cli**: Thin wrapper — parses args, calls store or API.
- **govrix-scout-reports**: Stateless template rendering. Takes data structs, returns bytes.

---

## Code Standards

### Rust

We use `rustfmt` for formatting and `clippy` for linting. Both are enforced in CI.

```bash
# Format
cargo fmt --all

# Lint (must produce zero warnings)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Type check
cargo check --workspace
```

Key rules enforced by `clippy`:

- All `Result` and `Option` values must be handled (no silent drops).
- No `unwrap()` or `expect()` in non-test code. Use `?` or explicit error handling.
- No `println!` in library code; use `tracing::info!` / `tracing::warn!` / `tracing::error!`.
- No hardcoded strings that belong in config.

The proxy hot path (anything called from `proxy/handler.rs` in the `service_fn`) must be `async`-safe and must not hold `Mutex` guards across `.await` points.

### TypeScript / React

We use ESLint + Prettier for the dashboard.

```bash
cd dashboard

# Format
pnpm prettier --write src/

# Lint
pnpm eslint src/

# Type-check
pnpm tsc --noEmit

# Build (the canonical verification)
pnpm run build
```

Rules:

- All props and API response types must be defined in `src/api/types.ts`. No `any`.
- Data fetching must go through the hooks in `src/api/hooks.ts` using TanStack Query.
- No direct `fetch()` calls in components. Use the typed API client in `src/api/client.ts`.
- Components must not import from other pages — shared UI lives in `src/components/`.

### SQL

- All migrations are plain `.sql` files in `migrations/`. They are applied in filename order by the Docker entrypoint.
- Use `IF NOT EXISTS` and `IF EXISTS` so migrations are idempotent.
- Do not add columns to existing tables in a new migration — add a new migration file.
- Never drop columns or tables in a migration without a corresponding `docs/adr/` entry.

---

## Conventional Commits

All commit messages must follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>

[optional body]

[optional footer]
```

**Types:**

| Type | When to use |
|------|------------|
| `feat` | A new user-facing feature |
| `fix` | A bug fix |
| `perf` | A performance improvement |
| `refactor` | Code change that is neither a fix nor a feature |
| `test` | Adding or updating tests |
| `docs` | Documentation only |
| `ci` | CI/CD pipeline changes |
| `chore` | Maintenance (dependency bumps, formatting) |
| `build` | Build system or tooling changes |

**Scopes** (optional but encouraged): `proxy`, `api`, `store`, `common`, `policy`, `dashboard`, `cli`, `reports`, `docker`, `migrations`

**Examples:**

```
feat(proxy): add streaming body tee for SSE responses
fix(store): handle NULL latency_ms in event list query
perf(proxy): replace Arc<Mutex<HashMap>> with DashMap for session tracker
docs: add A2A identity section to README
test(policy): add SSN false-positive edge cases
ci: pin rust-toolchain to 1.82 for reproducible builds
```

Breaking changes must include `BREAKING CHANGE:` in the commit footer:

```
feat(api): rename /api/v1/events/stream to /api/v1/events/live

BREAKING CHANGE: the SSE endpoint path has changed. Update any dashboard or client that was polling /stream.
```

---

## Pull Request Process

1. **Fork** the repository and create a branch from `main`.

2. **Branch naming**: `<type>/<short-slug>` — for example:
   - `feat/anthropic-streaming`
   - `fix/session-tracker-expiry`
   - `docs/api-reference`

3. **Keep PRs focused**. One logical change per PR. If you find unrelated issues while working, open a separate issue or PR.

4. **Write tests**. Every new function in Rust crates must have at least one unit test. New API endpoints must have handler tests.

5. **CI must pass**. The CI pipeline runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace`, and `pnpm run build`. All must be green before merge.

6. **Fill the PR template**. Describe what changed, why, and how to test it manually.

7. **One approving review required** from a maintainer.

8. **Squash and merge** is the preferred merge strategy to keep `main` history clean.

### PR Checklist

- [ ] `cargo fmt --all` has been run
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] `pnpm run build` passes (if dashboard changed)
- [ ] New functions have tests
- [ ] Documentation updated if behavior changed
- [ ] Conventional commit message on all commits
- [ ] No secrets, tokens, or credentials in the diff

---

## Issue Labels

| Label | Meaning |
|-------|---------|
| `bug` | Something is not working as documented |
| `enhancement` | New feature or improvement to an existing feature |
| `good first issue` | Suitable for first-time contributors |
| `help wanted` | Maintainer is actively looking for community help |
| `performance` | Latency, throughput, or memory usage concerns |
| `security` | Security-sensitive issue (see Security section below) |
| `documentation` | Documentation gap or error |
| `question` | Not a bug — needs clarification |
| `wontfix` | Intentional design decision, will not be changed |
| `duplicate` | Already tracked in another issue |
| `proxy` | Related to the proxy hot path |
| `dashboard` | Related to the React dashboard |
| `api` | Related to the REST API |
| `compliance` | Related to the compliance/policy engine |
| `breaking` | Would require a migration or behavior change for existing users |

---

## Architecture Decision Records

Significant design decisions are documented as Architecture Decision Records (ADRs) in `docs/adr/`.

ADR format (`docs/adr/NNNN-short-title.md`):

```markdown
# NNNN: Title

**Status**: Accepted | Superseded by NNNN | Deprecated

## Context

What is the situation that motivates this decision?

## Decision

What was decided?

## Consequences

What are the positive and negative consequences?
```

**Create an ADR when:**

- Choosing between two reasonable technical approaches (e.g., "why hyper not axum for the hot path")
- Changing a database schema column type or constraint
- Changing the event channel capacity or batch writer timing
- Changing the compliance-first invariants
- Introducing or removing a dependency

**Existing ADRs:**

- `docs/adr/0001-hyper-not-axum-for-proxy-hot-path.md` — Why the proxy uses raw hyper `service_fn` instead of axum
- `docs/adr/0002-fail-open-design.md` — Why the proxy never blocks traffic on internal errors
- `docs/adr/0003-runtime-sqlx-not-compile-time.md` — Why we use `sqlx::query()` instead of `query!` macros

---

## Testing Policy

| Layer | Framework | Location |
|-------|-----------|----------|
| Rust unit tests | `cargo test` | Inline `#[cfg(test)]` modules in each source file |
| Rust integration tests | `cargo test --test '*'` | `tests/` directory (requires live PostgreSQL) |
| Dashboard | Vitest | `dashboard/src/**/*.test.ts` |

**Rules:**

- Unit tests must not open network connections or write to disk.
- Integration tests must create their own isolated schema and clean up after themselves.
- Tests must be deterministic — no `sleep`, no time-dependent assertions without mocks.
- `cargo test --workspace` must pass without a running database (unit tests only).

---

## Security

Do not open a public GitHub issue for security vulnerabilities. Email `security@Govrix Scout.io` directly with:

- A description of the vulnerability
- Steps to reproduce
- The potential impact

We will acknowledge within 48 hours and aim to publish a patch within 14 days of confirmation.

**Particular attention areas:**

- The proxy must never log or store raw PII values — only type and field path.
- The compliance fields (`session_id`, `timestamp`, `lineage_hash`, `compliance_tag`) must be present on every event — no exceptions.
- The proxy must not add latency beyond 5ms p99 to the hot path.
- The fail-open invariant must hold — internal crashes must never block agent traffic.
