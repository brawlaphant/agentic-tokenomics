# Licensing

Copyright 2024-2026 Regen Network Development, PBC and Contributors.

This repository uses a **dual-license** model to appropriately cover both code and documentation.

---

## Code — Apache License 2.0

All source code, scripts, schemas, configuration files, and executable artifacts in this repository are licensed under the [Apache License, Version 2.0](LICENSE).

**SPDX Identifier:** `Apache-2.0`

### Covered files and directories

| Path | Description |
|------|-------------|
| `agents/` | ElizaOS agent scaffold (monorepo, plugins, tests) |
| `agent-002-governance-analyst/` | Standalone AGENT-002 reference implementation |
| `scripts/` | Verification and index tooling |
| `schemas/` | Shared JSON schemas |
| `simulations/` | cadCAD economic simulation code |
| `mechanisms/*/reference-impl/*.js` | Mechanism reference implementations |
| `mechanisms/*/schemas/*.json` | Mechanism JSON schemas |
| `mechanisms/*/datasets/schema.json` | Dataset schemas |
| `mechanisms/*/datasets/fixtures/*.json` | Test fixture data |

### Covered file types (anywhere in repo)

`*.js`, `*.mjs`, `*.ts`, `*.py`, `*.json`, `*.sql`, `*.yml`, `*.yaml`,
`docker-compose.yml`, `Dockerfile`, `package.json`, `package-lock.json`,
`tsconfig.json`, `.gitignore`, `.env.example`, `vitest.config.ts`

---

## Documentation — Creative Commons Attribution-ShareAlike 4.0 International

All documentation, specifications, design documents, and written content in this repository are licensed under [CC BY-SA 4.0](LICENSE-CC-BY-SA).

**SPDX Identifier:** `CC-BY-SA-4.0`

### Covered files and directories

| Path | Description |
|------|-------------|
| `docs/` | Architecture, economics, governance, integration, learning documentation |
| `phase-1/` through `phase-5/` | Phase specifications and design documents |
| `mechanisms/*/SPEC.md` | Mechanism specifications |
| `mechanisms/*/README.md` | Mechanism overviews |
| `mechanisms/*/datasets/README.md` | Dataset documentation |
| `mechanisms/*/reference-impl/README.md` | Reference implementation documentation |
| `mechanisms/*/schemas/README.md` | Schema documentation |

### Covered root files

`README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `CLAUDE.md`

---

## Mixed directories

Some directories contain both code and documentation:

- **`mechanisms/`** — `SPEC.md` and `README.md` files are CC-BY-SA 4.0; `*.js` and `*.json` files are Apache 2.0
- **`agents/`** — `README.md` files are CC-BY-SA 4.0; all other files are Apache 2.0

When in doubt, the file's **purpose** determines the license:

- **Documentation and specifications** are licensed under **CC-BY-SA 4.0**. This includes Markdown files (`.md`), diagrams (`.svg`, `.png`, `.jpg`), and other media that serve as part of design documents or specifications. For example, an `.svg` architecture diagram in `docs/` is CC-BY-SA 4.0.
- **Source code and configuration** are licensed under **Apache 2.0**. This covers all file types listed in the "Code" section above, as well as any executable or programmatic assets. For example, an `.svg` used as a UI component in application code is Apache 2.0.

---

## Contributions

By contributing to this repository, you agree that your contributions will be licensed under the applicable license for the type of content contributed:

- **Code contributions** (pull requests containing source code, scripts, schemas, or configuration) are licensed under Apache 2.0
- **Documentation contributions** (pull requests containing specifications, design documents, or other written content) are licensed under CC-BY-SA 4.0
- **Mixed contributions** are licensed under both licenses, with each file covered by the license corresponding to its type as described above

---

## Third-party dependencies

This repository may include or reference third-party software and content. Such materials are subject to their own license terms. See individual `package.json` files and dependency declarations for details.
