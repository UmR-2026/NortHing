# Reference Library

> **Read this first** before writing code in any of the 4 covered domains.

## Purpose

`.agents/reference/` is a **read-only code mirror** for the 4 most-touched implementation
domains in this project. Every entry is a faithful copy (or, where noted, an extracted
signature) of existing production code or design-document code, with the source
path and commit SHA recorded at the top of each file.

The goal: **stop re-deriving the same patterns from scratch.** When you write new
code in any covered domain, copy the relevant pattern from here first; if the
pattern has drifted from the reference, file a sync task.

## Domains

| Domain | Path | Status | Most-referenced files |
|---|---|---|---|
| **Skill / Registry / Loader** | [`skills/`](skills/) | Shipped (A4 done) | `05-skill-resolver-v2.rs`, `06-skill-registry-skeleton.rs` |
| **Actor / One-shot Dispatcher** | [`actor/`](actor/) | Designed, **not yet implemented** | `01-skill-actor-trait.md`, `04-coordinator-spawn-pattern.rs` |
| **Session / Multi-session / Coordinator** | [`session/`](session/) | Shipped (A6 done) | `01-conversation-coordinator.rs`, `06-app-state-slint-wiring.rs` |
| **Plan Compliance Checker** | [`checker/`](checker/) | Shipped (Phases 1–4 done) | `03-parse-plan.rs`, `04-check-plan.rs` |
| **Upstream (GitHub) references** | [`_upstream/`](_upstream/) | Optional | `tokio-actor-patterns.md` |

## File naming convention

| Pattern | Meaning |
|---|---|
| `NN-name.rs` | Full source-file mirror. `// REFERENCE — copied from <path>:<line>` at the top. |
| `NN-name.md` | Extracted code blocks, design-doc snippets, format specs. |
| `SIGNATURES.md` | One-page signature card. Function → signature → purpose → source line. |
| `NOTES.md` | "Do NOT copy verbatim" + DEPRECATED + known-gap list. **Read this second.** |
| `README.md` | Per-domain readme with selection guide, ordering, caveats. |

## Mandatory workflow before writing code in a covered domain

1. Read the domain's `README.md`.
2. Read the domain's `SIGNATURES.md` to find the right function/trait.
3. Read the domain's `NOTES.md` to see what NOT to copy.
4. Open the specific `NN-*.rs` and copy the pattern (or quote it in your new file's
   doc comment with `// Pattern source: .agents/reference/<domain>/0N-xxx.rs`).
5. If you change the upstream pattern, update the mirror in the same commit.

## Maintenance

- **First synced**: 2026-06-19 (commit on `v3-restructure`).
- **Last synced** is recorded in each `NN-*.rs` header.
- When a commit lands in `src/` that materially changes a covered pattern, the
  corresponding mirror file must be updated in the **same** commit (or a follow-up
  `docs(reference): re-sync after <sha>` commit).

## What this directory is NOT

- **Not** a cargo workspace member. Nothing here compiles. Linters should ignore it.
- **Not** an entry point. New users should not read these files unprompted.
- **Not** a public API. Comments and code may be reduced for clarity; production
  behavior lives in `src/`.
