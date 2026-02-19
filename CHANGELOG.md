# Changelog

All notable changes to Govrix Platform will be documented in this file.

## [Unreleased]

## [0.1.0] - 2026-02-19

### Added
- govrix-common: PlatformConfig, LicenseTier (Community/Starter/Growth/Enterprise), TenantRegistry
- govrix-policy: PolicyEngine with YAML rule loading and hot-reload, PII masking, BudgetTracker
- govrix-policy: GovrixPolicyHook bridging Scout's PolicyHook trait to Govrix engine
- govrix-identity: Certificate Authority generation, agent mTLS cert issuance, MtlsConfig
- govrix-server: Full startup pipeline with license validation, budget wiring, mTLS TLS listener
- govrix-server: REST API (7 endpoints: health, license, policies, reload, tenants, certs/issue)
- govrix-keygen: CLI binary to mint and validate license keys
- Tier-based budget defaults (Starter: 50M tokens/$500, Growth: 500M/$5000, Enterprise: unlimited)
- Per-agent token and cost budget limits via config
- Docker and docker-compose support (ports 4000/4001/4443)
- Kubernetes manifests with kustomization
- GitHub Actions CI
