# Development Environment Setup

This guide covers setting up a local development environment for the Regen Agentic Tokenomics project, including mechanism development, smart contract compilation, agent runtime, and local chain testing.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Clone and Install](#clone-and-install)
3. [Running Reference Implementations](#running-reference-implementations)
4. [Local Cosmos SDK (Regen Ledger)](#local-cosmos-sdk-regen-ledger)
5. [Local CosmWasm Development](#local-cosmwasm-development)
6. [Local ElizaOS Development](#local-elizaos-development)
7. [Local Chain (Single-Node Testnet)](#local-chain-single-node-testnet)
8. [Running Tests](#running-tests)
9. [Docker Compose Environment](#docker-compose-environment)
10. [Troubleshooting](#troubleshooting)

---

## Prerequisites

Install the following tools before proceeding:

| Tool | Minimum Version | Purpose |
|------|-----------------|---------|
| **Node.js** | 18+ | Mechanism reference implementations, scripts |
| **Go** | 1.21+ | Cosmos SDK modules, regen-ledger |
| **Rust** | 1.75+ | CosmWasm smart contracts |
| **Docker** | 24+ | Containerized development, testnet |
| **Docker Compose** | 2.20+ | Multi-service orchestration (chain + agents + databases) |
| **Git** | 2.30+ | Version control |

### Optional but Recommended

| Tool | Purpose |
|------|---------|
| **jq** | JSON processing in scripts |
| **buf** | Protobuf linting and code generation |
| **cargo-generate** | CosmWasm contract scaffolding |
| **cosmwasm-check** | Wasm binary validation |
| **wasm-opt** | WebAssembly binary optimization |
| **golangci-lint** | Go linting |
| **protoc** | Protobuf compilation for Cosmos SDK modules |

### Verify Versions

```bash
node --version    # v18.x or higher
go version        # go1.21 or higher
rustc --version   # 1.75.0 or higher
docker --version  # 24.x or higher
```

---

## Clone and Install

```bash
# Clone the repository (or your fork)
git clone https://github.com/regen-network/agentic-tokenomics.git
cd agentic-tokenomics

# Install Node.js dependencies
npm install

# Verify the repo structure and schemas
node scripts/verify.mjs

# Check the mechanism index is up to date
node scripts/build-mechanism-index.mjs --check
```

If you are contributing via fork:

```bash
git clone https://github.com/<your-username>/agentic-tokenomics.git
cd agentic-tokenomics
git remote add upstream https://github.com/regen-network/agentic-tokenomics.git
npm install
```

### Available npm Scripts

| Script | Command | Purpose |
|--------|---------|---------|
| `verify` | `node scripts/verify.mjs` | Validate required files, schemas, mechanism index |
| `build:index` | `node scripts/build-mechanism-index.mjs` | Regenerate mechanism index in README.md |
| `verify:m010:datasets` | `node scripts/verify-m010-datasets.mjs` | Validate M010 dataset fixtures |
| `check:index` | `node scripts/build-mechanism-index.mjs --check` | Verify mechanism index is current (CI-safe) |

---

## Running Reference Implementations

Each mechanism in `mechanisms/` includes a JavaScript reference implementation with test vectors.

```bash
# Run the full verification suite
node scripts/verify.mjs

# Run a specific mechanism's reference implementation (example: m010)
node mechanisms/m010-reputation-signal/reference-impl/m010_score.js

# Test vectors are in reference-impl/test_vectors/
# Input:    vector_v0_sample.input.json
# Expected: vector_v0_sample.expected.json

# Run M012 supply algorithm self-test
node mechanisms/m012-fixed-cap-dynamic-supply/reference-impl/m012_supply.js

# Run M013 fee routing self-test
node mechanisms/m013-value-based-fee-routing/reference-impl/m013_fee.js

# Run M015 contribution rewards self-test
node mechanisms/m015-contribution-weighted-rewards/reference-impl/m015_score.js
```

### Adding New Test Vectors

1. Create an input file: `mechanisms/<id>/reference-impl/test_vectors/vector_<scenario>.input.json`
2. Create the expected output: `mechanisms/<id>/reference-impl/test_vectors/vector_<scenario>.expected.json`
3. Ensure the reference implementation produces the expected output when given the input
4. Run `node scripts/verify.mjs` to confirm all vectors pass

---

## Local Cosmos SDK (Regen Ledger)

### Install regen-ledger

```bash
# Clone regen-ledger
git clone https://github.com/regen-network/regen-ledger.git
cd regen-ledger

# Checkout a compatible version
git checkout v6.0.0  # or latest stable tag

# Build from source
make install

# Verify installation
regen version
```

### Build Custom Modules

If you are developing new Cosmos SDK modules for mechanisms (e.g. `x/supply` for M012, `x/attestation` for M008):

```bash
cd regen-ledger

# Build all modules
make build

# Run Go unit tests
go test ./x/ecocredit/...
go test ./x/data/...

# Test a specific new module
go test ./x/<module-name>/... -v

# Lint
golangci-lint run ./x/<module-name>/...

# Generate protobuf types (requires buf or protoc)
make proto-gen

# Lint proto files
buf lint proto/
```

### Module Development Pattern

New modules follow the Cosmos SDK keeper pattern:
- `x/<module>/keeper/` -- State management and business logic
- `x/<module>/types/` -- Message types, protobuf definitions
- `x/<module>/client/` -- CLI and REST client
- `x/<module>/module.go` -- Module registration

---

## Local CosmWasm Development

### Install the Wasm Toolchain

```bash
# Add the wasm32 target
rustup target add wasm32-unknown-unknown

# Install cargo-generate for contract scaffolding (optional)
cargo install cargo-generate

# Install cosmwasm-check for binary validation (optional)
cargo install cosmwasm-check
```

### Compile Contracts

```bash
# Navigate to a contract directory (example)
cd contracts/attestation-bond

# Build the contract
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown

# The compiled wasm binary will be at:
# target/wasm32-unknown-unknown/release/<contract_name>.wasm

# Validate the binary
cosmwasm-check target/wasm32-unknown-unknown/release/attestation_bond.wasm
```

### Optimized Production Build

For deployment, use the CosmWasm optimizer Docker image (recommended for reproducible builds):

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.0
```

### Run Contract Tests

```bash
# Unit tests
cargo test

# Unit tests with output
cargo test -- --nocapture

# Run clippy for lint checks
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt -- --check
```

### Contract Directory Structure

Each contract follows the CosmWasm convention:

```
contracts/<mechanism-name>/
  Cargo.toml
  src/
    contract.rs      # Entry points (instantiate, execute, query)
    msg.rs           # Message types (InstantiateMsg, ExecuteMsg, QueryMsg)
    state.rs         # Storage definitions
    error.rs         # Error types
    lib.rs           # Module declarations
  tests/
    integration.rs   # Multi-contract integration tests
```

---

## Local ElizaOS Development

### Install ElizaOS

```bash
# Clone ElizaOS
git clone https://github.com/elizaos/eliza.git
cd eliza

# Install dependencies
pnpm install

# Build the project
pnpm build
```

### Configure Plugins

Agent characters for this project use the following ElizaOS plugins:

| Plugin | Purpose |
|--------|---------|
| `@elizaos/plugin-knowledge` | Knowledge graph integration via KOI MCP |
| `@elizaos/plugin-cosmos` | Regen Ledger on-chain queries |
| `@elizaos/plugin-mcp` | Model Context Protocol integration |

Configure plugin access in the agent character file:

```yaml
# characters/registry-reviewer.yaml
name: "Registry Reviewer"
plugins:
  - "@elizaos/plugin-knowledge"
  - "@elizaos/plugin-cosmos"
  - "@elizaos/plugin-mcp"

mcp_servers:
  koi:
    endpoint: "http://localhost:3100"
  ledger:
    endpoint: "http://localhost:3200"
  tx_builder:
    endpoint: "http://localhost:3300"
```

### Run Agents Locally

```bash
# Start with a specific character
pnpm start --character characters/registry-reviewer.yaml

# Start with environment configuration
REGEN_NODE_URL=http://localhost:26657 \
KOI_MCP_URL=http://localhost:3100 \
pnpm start --character characters/registry-reviewer.yaml
```

### MCP Server Development

For developing or testing MCP integrations:

```bash
# Start the KOI MCP server locally
cd koi-research
pnpm run mcp:start

# Start the Ledger MCP server locally (requires a running regen node)
cd regen-ledger-mcp
pnpm run start -- --node http://localhost:26657
```

### Agent Character Files

Create agent character configs based on specifications in `phase-2/2.4-agent-orchestration.md`:

| Character File | Agent ID | Role |
|----------------|----------|------|
| `registry-reviewer.yaml` | AGENT-001 | Credit class/project/batch review |
| `governance-analyst.yaml` | AGENT-002 | Proposal analysis, voting prediction |
| `market-monitor.yaml` | AGENT-003 | Price/liquidity/retirement monitoring |
| `validator-monitor.yaml` | AGENT-004 | Validator performance, delegation flows |

---

## Local Chain (Single-Node Testnet)

### Quick Start

```bash
# Remove previous state (if any)
rm -rf ~/.regen

# Initialize the chain
regen init test-node --chain-id regen-local-1

# Create a validator key
regen keys add validator --keyring-backend test

# Add genesis account with tokens
regen genesis add-genesis-account validator 100000000000uregen --keyring-backend test

# Generate the genesis transaction
regen genesis gentx validator 10000000000uregen \
  --chain-id regen-local-1 \
  --keyring-backend test

# Collect genesis transactions
regen genesis collect-gentxs

# Configure minimum gas prices
sed -i'' -e 's/minimum-gas-prices = ""/minimum-gas-prices = "0.025uregen"/' \
  ~/.regen/config/app.toml

# Enable API and gRPC
sed -i'' -e '/\[api\]/,/\[/{s/enable = false/enable = true/}' \
  ~/.regen/config/app.toml
sed -i'' -e '/\[grpc\]/,/\[/{s/enable = false/enable = true/}' \
  ~/.regen/config/app.toml

# Start the node
regen start
```

### Genesis Configuration for Testing

To pre-configure the genesis state with additional test accounts:

```bash
# Add additional test accounts
regen keys add alice --keyring-backend test
regen keys add bob --keyring-backend test

regen genesis add-genesis-account alice 50000000000uregen --keyring-backend test
regen genesis add-genesis-account bob 50000000000uregen --keyring-backend test
```

### Useful Local Chain Commands

```bash
# Check node status
regen status

# Query account balance
regen query bank balances $(regen keys show validator -a --keyring-backend test)

# Query ecocredit module state
regen query ecocredit classes

# Submit a governance proposal
regen tx gov submit-proposal <proposal.json> \
  --from validator --keyring-backend test \
  --chain-id regen-local-1 --yes

# Store a CosmWasm contract
regen tx wasm store contract.wasm \
  --from validator --keyring-backend test \
  --chain-id regen-local-1 --gas auto --gas-adjustment 1.3 --yes
```

---

## Running Tests

### Go Tests (Cosmos SDK Modules)

```bash
cd regen-ledger

# All tests
go test ./...

# Specific module with verbose output
go test ./x/ecocredit/... -v

# With coverage
go test ./x/ecocredit/... -coverprofile=coverage.out
go tool cover -html=coverage.out
```

### Rust Tests (CosmWasm Contracts)

```bash
cd contracts/attestation-bond

# Unit tests
cargo test

# Unit tests with output
cargo test -- --nocapture

# Integration tests only
cargo test --test integration

# All contracts at once
cd contracts && cargo test --workspace

# Coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### TypeScript / JavaScript Tests

```bash
# Repository-level verification
node scripts/verify.mjs

# Mechanism-specific tests (if present)
node mechanisms/m010-reputation-signal/reference-impl/m010_score.js
```

### Integration Tests

```bash
# Start the local stack with Docker
docker compose -f docker-compose.testnet.yml up -d

# Wait for chain to produce blocks
sleep 10

# Run the integration test suite
npm run test:integration

# Tear down
docker compose -f docker-compose.testnet.yml down
```

### End-to-End Tests

E2E tests exercise full workflows from agent trigger through on-chain execution:

```bash
# Start the full stack (chain + agents + MCP servers)
docker compose -f docker-compose.testnet.yml up -d

# Run E2E test suite
npm run test:e2e

# View logs for debugging
docker compose -f docker-compose.testnet.yml logs -f agent-runtime
```

### Test Coverage Targets

| Layer | Tool | Target |
|-------|------|--------|
| Smart contracts (Rust) | `cargo tarpaulin` | 90% |
| SDK modules (Go) | `go test -cover` | 80% |
| Reference implementations (JS) | Test vectors | All vectors pass |
| Integration | Custom harness | Critical paths |
| E2E | Workflow harness | Workflow completion |

---

## Docker Compose Environment

### Full Testnet Stack

The Docker Compose configuration starts a complete development environment:

```bash
# Start all services
docker compose -f docker-compose.testnet.yml up -d

# View running services
docker compose -f docker-compose.testnet.yml ps

# View logs
docker compose -f docker-compose.testnet.yml logs -f

# Stop and clean up
docker compose -f docker-compose.testnet.yml down

# Stop and also remove volumes (full reset)
docker compose -f docker-compose.testnet.yml down -v
```

### Services

| Service | Port | Description |
|---------|------|-------------|
| `regen-node` | 26657 (RPC), 1317 (API), 9090 (gRPC) | Local Regen Ledger node |
| `postgres` | 5432 | Agent state and memory storage (pgvector) |
| `redis` | 6379 | Event streaming and cache (Redis Streams) |
| `jena` | 3030 | Apache Jena Fuseki (KOI knowledge graph) |
| `koi-mcp` | 3100 | KOI knowledge graph MCP server |
| `ledger-mcp` | 3200 | Regen Ledger MCP server |
| `agent-runtime` | 3000 | ElizaOS agent runtime |

### Build Custom Images

```bash
# Build all images
docker compose -f docker-compose.testnet.yml build

# Build a specific service
docker compose -f docker-compose.testnet.yml build agent-runtime

# Force rebuild without cache
docker compose -f docker-compose.testnet.yml build --no-cache
```

---

## Troubleshooting

### `node scripts/verify.mjs` Fails with "Missing required file"

The verify script checks for specific files in the mechanism directories. Ensure all required files exist:
- `mechanisms/<id>/SPEC.md`
- `mechanisms/<id>/README.md`
- `mechanisms/<id>/schemas/`
- `mechanisms/<id>/reference-impl/`
- `mechanisms/<id>/datasets/`

### Mechanism Index Out of Date

```
Error: Mechanism index is stale
```

Regenerate the index:
```bash
node scripts/build-mechanism-index.mjs
```

### Rust Wasm Compilation Errors

```
error[E0463]: can't find crate for `std`
```

Ensure the wasm32 target is installed:
```bash
rustup target add wasm32-unknown-unknown
```

### Go Module Issues

```
go: module not found
```

Ensure you are using the correct Go version and your module cache is clean:
```bash
go clean -modcache
go mod tidy
go mod download
```

### Docker: Port Already in Use

```
Error: Bind for 0.0.0.0:26657 failed: port is already allocated
```

Stop the conflicting service or change the port mapping in `docker-compose.testnet.yml`:
```bash
# Find what is using the port
lsof -i :26657

# Kill the process if needed
kill -9 <PID>
```

### Local Chain: Genesis Error

```
Error: genesis.json file already exists
```

Remove existing state and reinitialize:
```bash
rm -rf ~/.regen
regen init test-node --chain-id regen-local-1
```

### ElizaOS Plugin Not Found

Ensure you have the required plugins installed in your ElizaOS workspace:
```bash
cd eliza
pnpm install @elizaos/plugin-knowledge @elizaos/plugin-cosmos @elizaos/plugin-mcp
pnpm build
```

### macOS: sed In-Place Editing

On macOS, `sed -i` requires an empty string argument for the backup extension. The commands in this guide use `sed -i''` which works on macOS. On Linux, use `sed -i` without the empty quotes.

### CosmWasm: Contract Binary Too Large

Use the CosmWasm optimizer Docker image for production-size binaries:
```bash
docker run --rm -v "$(pwd)":/code \
  cosmwasm/optimizer:0.16.0
```

### Schema Validation Errors

Ensure your JSON Schema files target draft 2020-12 and include a `$id` field:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schema.regen.network/m0XX/message-name.json",
  "type": "object",
  "properties": {}
}
```

### Permission Errors on macOS

If Docker volume mounts fail on macOS, ensure Docker Desktop has file sharing enabled for your project directory in Settings > Resources > File Sharing.

---

*This document is part of the Regen Network Agentic Tokenomics framework.*
