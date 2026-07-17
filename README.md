# northhing

A general-purpose agent application with a Slint-based desktop interface. The IDE/CLI/coding capabilities are tools for the agent — not a human-facing IDE.

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
pnpm run desktop:dev          # build and run Slint desktop app (cold start)
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

**Shipping (v0.1.0)**: Slint desktop + installer.  
**Frozen-experimental**: CLI, server, relay, mobile-web, MiniApp UI, SDLC harness.

## Tech Debt

See [`docs/status/tech-debt-ledger.md`](docs/status/tech-debt-ledger.md) for the living tech debt ledger.

## License

See [`LICENSE`](LICENSE).
