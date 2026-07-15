# CodeGraph + context7 MCP integration — setup handoff

> **Date**: 2026-06-25
> **Goal**: wire pre-indexed code knowledge graph + live library docs into the NortHing agent runtime so it stops spending tokens on exploration (CodeGraph) and on stale training data (context7).

---

## TL;DR

| Tool | Solves | Cost | Setup |
|---|---|---|---|
| **CodeGraph** | "How is *my* code organized?" (local, instant, symbol-level graph) | Free, local SQLite | `codegraph init -i` + register as stdio MCP server |
| **context7** | "What's the current API of *library X*?" (remote, current docs) | Free tier w/ API key | Register as stdio MCP server |

Both expose MCP tools that the NortHing agent can call directly. Both already index / build on first run.

---

## 1. Why both?

- **CodeGraph** indexes the local repo (tree-sitter → SQLite). `codegraph_callers`, `codegraph_impact`, `codegraph_context` answer questions about *this* codebase in O(1) — no file scans.
- **context7** indexes upstream libraries (Next.js, React Query, axum, tokio…). `get-library-docs` returns version-specific snippets that get injected into the prompt.

The combination covers both internal navigation and external API lookup. They're complementary, not redundant.

---

## 2. CodeGraph — installed + working

Verified 2026-06-25 against the current `main` branch (HEAD `f6b00e7`):

```
.codegraph/codegraph.db  →  120 MB  (27 Rust crates + 7 frontend packages indexed)
```

### CLI commands (verified output)

```bash
# Symbol search by name
npx @colbymchenry/codegraph query "tick" --kind function
# → counting_actor_records_ticks (actor.rs:216)
# → round_tick_result_variants_match_semantics (execution_engine.rs:3441)
# → auto_save_interval_waits_before_first_tick (session_manager.rs:4512)
# → periodic_actor_ticks_repeatedly_and_stops_on_cancel (runtime.rs:758)

# Call-chain / dependency exploration
npx @colbymchenry/codegraph explore "coordinator execute_hidden_subagent"
# → 215 symbols across 84 files, with blast radius + relationships

# Impact analysis
npx @colbymchenry/codegraph impact "ExecutionEngine::tick"
# → 5 affected symbols: tick, execute_dialog_turn_impl, execute_dialog_turn
#   (execution_engine.rs) + tick (a1_path.rs:172) + tick (long_running.rs:87)
```

Note: `--json` flag is not supported in this version (returns `unknown option '--json'`); use the default table output.

### Auto-sync

CodeGraph uses Windows ReadDirectoryChangesW to watch the workspace; after a 2s debounce window it incrementally re-indexes changed source files. No daemon to manage.

---

## 3. MCP server registration

The desktop MCP integration was fixed in `4af311c` (P0-D). To add the two servers, edit the user's `~/.northhing/config/app.json` (or write the file if it doesn't exist yet — it gets created on first successful launch).

### CodeGraph (no API key required)

```jsonc
{
  "mcp": {
    "servers": {
      "codegraph": {
        "transport": "stdio",
        "command": "npx",
        "args": ["-y", "@colbymchenry/codegraph", "mcp"],
        "cwd": "E:/agent-project/northing"
      }
    }
  }
}
```

The `cwd` is critical — CodeGraph's MCP server reads `.codegraph/codegraph.db` from the working directory. Without it, the server starts but every query returns "no index found".

### context7 (API key required, free tier)

```jsonc
{
  "mcp": {
    "servers": {
      "context7": {
        "transport": "stdio",
        "command": "cmd",
        "args": [
          "/c",
          "npx",
          "-y",
          "@upstash/context7-mcp",
          "--api-key",
          "<YOUR_CONTEXT7_API_KEY>"
        ]
      }
    }
  }
}
```

The `cmd /c` wrapper is required on Windows — `npx` is a `.bat` and stdio MCP needs a real process. Without it the server fails to attach stdio. (See known Windows-on-context7 issue: `claude mcp list` shows `Failed to connect`.)

Get the API key at https://context7.com (free, no payment required for personal use).

### Verifying both are live

After registering, launch the desktop binary. The status bar's MCP segment should show:

```
MCP: 2 servers (codegraph, context7)
```

If only one shows up, check `~/.northhing/logs/` for the server-manager init trace.

---

## 4. Available MCP tools after registration

### From CodeGraph (8)

```
codegraph_search         name-based symbol search
codegraph_context        task-aware code context builder (replaces "explore sub-agent")
codegraph_callers        who calls this symbol
codegraph_callees        what this symbol calls
codegraph_impact         blast radius of a change
codegraph_node           single-symbol details
codegraph_files          indexed file tree (faster than fs scan)
codegraph_status         index health + counts
```

### From context7 (2)

```
resolve-library-id       map common name → canonical Context7 ID
query-docs               fetch version-specific docs + examples
```

---

## 5. `.gitignore` change (committed)

`.codegraph/` is now in the project root `.gitignore` (this commit). The nested `.codegraph/.gitignore` already excludes `*` except itself; the root entry skips the directory itself so `git status` doesn't list it as untracked.

Each developer's machine rebuilds the index on first `codegraph init -i`. The 120 MB SQLite DB is local state, never version-controlled.

---

## 6. Expected payoff

Per the upstream benchmark (VS Code · ~10k TypeScript files, the closest reference point to a 27-crate Rust workspace):

| Metric | Without | With | Δ |
|---|---|---|---|
| Tool calls per architecture question | 23 | 7 | **−70%** |
| Tokens consumed | 1.4M | 390k | **−73%** |
| Wall time | baseline | −41% | **−41%** |

Real-world numbers for NortHing will differ (smaller repo, Rust instead of TS) but the order-of-magnitude improvement is the headline. The agent's "exploration tax" — what we measured as the agent spending 30+ min grepping for symbols — drops dramatically.

For Mavis specifically: when I'm asked to refactor `app_state/mod.rs` or debug `AIClientFactory`, I now have `codegraph_callers` / `codegraph_context` as my first call instead of `grep -r` across the workspace.

---

## 7. Open items

- [ ] `~/.northhing/config/app.json` doesn't exist yet (app never ran successfully to first-boot-create it). Needs to be written manually OR the desktop binary needs a successful first launch.
- [ ] Status bar MCP segment currently reads "Pending" — needs the global MCP registration fix from `4af311c` to take effect on next launch.
- [ ] context7 API key: register and fill in `<YOUR_CONTEXT7_API_KEY>` above before adding to `app.json`.
- [ ] Verify the two servers appear in the MCP status segment after launch.