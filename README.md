# NortHing

An **agent-first growth container** — see [`docs/product/charter.md`](docs/product/charter.md). The desktop app is a vessel where an agent (「北」) grows; humans intervene by speech only, memory and logs are inviolable by human hands, and total destruction is the only absolute human power. The IDE/CLI/coding capabilities are tools for the agent — not a human-facing IDE.

## Install

**End users**: Use the installer from the [Releases page](../../releases) or build it:

```bash
pnpm run installer:build
```

See [`northing-installer/README.md`](northing-installer/README.md) for details.

## Quick Start

1. **Install** the desktop app via the installer.
2. **Launch** — the welcome screen guides you through provider configuration.
3. **Configure a provider** (API key + base URL + model). Test credentials are not stored in the repo.
4. **Start chatting** with the agent.

## Development

```bash
pnpm run desktop:dev          # build and run Dioxus consult-room desktop app (cold start)
pnpm run desktop:check        # compile check only
pnpm run cli:dev              # run CLI (frozen surface)
pnpm run installer:build      # build installer
pnpm run e2e:test:chat        # run chat E2E tests
```

For the full script list, see [`package.json`](package.json).

## Architecture

See [`AGENTS.md`](AGENTS.md) for the layered module index, backbone invariants, and verification table.

### Surface status

See [`docs/status/surfaces.md`](docs/status/surfaces.md) for the complete ledger of shipping vs frozen-experimental surfaces.

**Shipping (v0.1.0)**: Dioxus desktop (consult-room) + installer.  
**Frozen-experimental**: CLI, server, SDLC harness.

## Tech Debt

See [`docs/status/tech-debt-ledger.md`](docs/status/tech-debt-ledger.md) for the living tech debt ledger.

## License

See [`LICENSE`](LICENSE).
