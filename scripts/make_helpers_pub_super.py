"""Add `pub(super)` to struct fields and private helpers in service.rs.

Reason: sibling lifecycle.rs `impl WorkspaceService` block needs to access
parent module's private helpers and struct fields. Rust default private
visibility does NOT extend across sibling files (only to descendants of the
declaring module). Therefore, fields and helpers must be at least
`pub(super)` to be reachable from `crate::service::workspace::lifecycle`.

The 13 lifecycle methods in sibling preserve their body content verbatim;
this script only adjusts visibility on the private helpers they call, plus
the struct fields they read (manager, path_manager, persistence, etc.).
"""
import sys

PATH = "src/crates/assembly/core/src/service/workspace/service.rs"


def read_file():
    with open(PATH, "r", encoding="utf-8") as f:
        return f.read()


def write_file(content):
    with open(PATH, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)


# --- Struct fields (L33-41) — add `pub(super)` so siblings can read them ---
FIELD_EDITS = [
    (
        "    manager: Arc<RwLock<WorkspaceManager>>,",
        "    pub(super) manager: Arc<RwLock<WorkspaceManager>>,",
    ),
    (
        "    config: WorkspaceManagerConfig,",
        "    pub(super) config: WorkspaceManagerConfig,",
    ),
    (
        "    persistence: Arc<PersistenceService>,",
        "    pub(super) persistence: Arc<PersistenceService>,",
    ),
    (
        "    path_manager: Arc<PathManager>,",
        "    pub(super) path_manager: Arc<PathManager>,",
    ),
    (
        "    runtime_service: Arc<WorkspaceRuntimeService>,",
        "    pub(super) runtime_service: Arc<WorkspaceRuntimeService>,",
    ),
]

# --- Helper methods — add `pub(super)` to the fn/async fn line ---
# Match the leading "    fn " or "    async fn " (4-space indent, since these
# are inside impl block) and prepend "pub(super) ". We use targeted line
# replacements keyed on the function name to avoid touching unrelated
# methods (e.g. the 13 facade delegates we just inserted, which are `pub
# async fn` already).
HELPER_EDITS = [
    # In r23a edit zone (L107-204):
    (
        "    fn collect_startup_restored_workspaces(",
        "    pub(super) fn collect_startup_restored_workspaces(",
    ),
    (
        "    fn push_startup_restored_workspace(",
        "    pub(super) fn push_startup_restored_workspace(",
    ),
    (
        "    async fn prepare_startup_restored_workspaces(",
        "    pub(super) async fn prepare_startup_restored_workspaces(",
    ),
    (
        "    async fn ensure_workspace_gitignore_best_effort(",
        "    pub(super) async fn ensure_workspace_gitignore_best_effort(",
    ),
    (
        "    async fn ensure_workspace_runtime_best_effort(",
        "    pub(super) async fn ensure_workspace_runtime_best_effort(",
    ),
    # Outside r23a zone (r23c/r23d territory) — required for sibling access:
    (
        "    async fn save_workspace_data(",
        "    pub(super) async fn save_workspace_data(",
    ),
    (
        "    async fn load_workspace_history_only(",
        "    pub(super) async fn load_workspace_history_only(",
    ),
    (
        "    fn to_manager_open_options(",
        "    pub(super) fn to_manager_open_options(",
    ),
    (
        "    fn assistant_display_name(",
        "    pub(super) fn assistant_display_name(",
    ),
    (
        "    async fn generate_assistant_workspace_id(",
        "    pub(super) async fn generate_assistant_workspace_id(",
    ),
    (
        "    async fn remap_legacy_assistant_workspace_records(",
        "    pub(super) async fn remap_legacy_assistant_workspace_records(",
    ),
    (
        "    fn normalize_workspace_options_for_path(",
        "    pub(super) fn normalize_workspace_options_for_path(",
    ),
    (
        "    async fn ensure_assistant_workspaces(",
        "    pub(super) async fn ensure_assistant_workspaces(",
    ),
    # The two additional helpers at L1890/L1920 are private test-only helpers
    # not called from sibling methods; skip them to minimize cross-zone edits.
]


def main():
    content = read_file()
    applied = 0
    missed = []
    for old, new in FIELD_EDITS + HELPER_EDITS:
        if old in content:
            content = content.replace(old, new, 1)
            applied += 1
        else:
            missed.append(old[:60])
    if missed:
        print(f"ERROR: {len(missed)} edits not found:")
        for m in missed:
            print(f"  - {m}")
        sys.exit(1)
    write_file(content)
    print(f"Applied {applied} visibility edits.")


if __name__ == "__main__":
    main()