# northhing English Language Pack
# English (US) (en-US) Fluent Translation File

# ==================== General ====================
app-version = Version { $version }
loading = Loading...
welcome = Welcome to northhing

# ==================== Actions ====================
action-confirm = Confirm
action-cancel = Cancel
action-save = Save
action-delete = Delete
action-edit = Edit
action-create = Create
action-add = Add
action-remove = Remove
action-close = Close
action-open = Open
action-copy = Copy
action-paste = Paste
action-undo = Undo
action-redo = Redo
action-refresh = Refresh
action-search = Search
action-retry = Retry
action-stop = Stop
action-start = Start

# ==================== Status ====================
status-loading = Loading
status-saving = Saving
status-saved = Saved
status-success = Success
status-error = Error
status-warning = Warning
status-info = Info
status-pending = Pending
status-processing = Processing
status-completed = Completed
status-failed = Failed
status-cancelled = Cancelled
status-ready = Ready
status-connected = Connected
status-disconnected = Disconnected

# ==================== File ====================
file-not-found = File not found: { $path }
file-read-error = Failed to read file: { $path }
file-write-error = Failed to write file: { $path }
file-delete-error = Failed to delete file: { $path }
file-permission-denied = Permission denied: { $path }
file-already-exists = File already exists: { $path }
file-saved = File saved: { $path }
file-created = File created: { $path }
file-deleted = File deleted: { $path }

# ==================== Workspace ====================
workspace-opened = Workspace opened: { $path }
workspace-closed = Workspace closed
workspace-not-found = Workspace not found
workspace-open-error = Failed to open workspace

# ==================== Git ====================
git-not-repository = Current directory is not a Git repository
git-commit-success = Commit successful
git-push-success = Push successful
git-pull-success = Pull successful
git-clone-error = Failed to clone repository
git-commit-error = Commit failed
git-push-error = Push failed
git-pull-error = Pull failed
git-merge-conflict = Merge conflict exists
git-branch-created = Branch created: { $name }
git-branch-deleted = Branch deleted: { $name }
git-checkout-success = Switched to branch: { $name }

# ==================== AI ====================
ai-connection-error = Failed to connect to AI service
ai-api-key-invalid = Invalid API key
ai-model-not-found = Model not found: { $model }
ai-context-too-long = Context exceeds limit
ai-rate-limited = Rate limit exceeded
ai-generation-error = Failed to generate content
ai-thinking = Thinking...
ai-generating = Generating...

# ==================== Terminal ====================
terminal-created = Terminal created
terminal-closed = Terminal closed
terminal-create-error = Failed to create terminal
terminal-command-error = Failed to execute command
terminal-shell-not-found = Shell not found

# ==================== Config ====================
config-loaded = Configuration loaded
config-saved = Configuration saved
config-load-error = Failed to load configuration
config-save-error = Failed to save configuration
config-invalid = Invalid configuration format
config-reset = Configuration reset

# ==================== Snapshot ====================
snapshot-created = Snapshot created: { $name }
snapshot-restored = Snapshot restored: { $name }
snapshot-deleted = Snapshot deleted
snapshot-create-error = Failed to create snapshot
snapshot-restore-error = Failed to restore snapshot
snapshot-not-found = Snapshot not found

# ==================== I18n ====================
language-changed = Language changed to: { $language }
language-not-supported = Unsupported language: { $language }

# ==================== Notifications ====================
notification-copied = Copied to clipboard
notification-settings-saved = Settings saved
notification-connection-established = Connection established
notification-connection-lost = Connection lost

# ==================== Errors ====================
error-unknown = An unknown error occurred
error-network = Network error
error-timeout = Request timeout
error-server = Server error
error-unauthorized = Unauthorized
error-forbidden = Access forbidden

# ==================== Time ====================
time-just-now = just now
time-seconds-ago = { $count } { $count ->
    [one] second
   *[other] seconds
} ago
time-minutes-ago = { $count } { $count ->
    [one] minute
   *[other] minutes
} ago
time-hours-ago = { $count } { $count ->
    [one] hour
   *[other] hours
} ago
time-days-ago = { $count } { $count ->
    [one] day
   *[other] days
} ago
time-weeks-ago = { $count } { $count ->
    [one] week
   *[other] weeks
} ago
time-months-ago = { $count } { $count ->
    [one] month
   *[other] months
} ago
time-years-ago = { $count } { $count ->
    [one] year
   *[other] years
} ago

# ==================== Dioxus Consult Room (R3' migration) ====================
dioxus-room-window-title = Consult Room · Active
dioxus-room-inner-window-title = Inner · The Self
dioxus-room-outer-window-title = Outer · The World
dioxus-room-state-drive = State: Drive
dioxus-room-status-identity = Know Sequence · Present
dioxus-room-status-context = architect_sub intervening
dioxus-room-head-name = Know Sequence
dioxus-room-head-initial = Seq
dioxus-room-head-state = Drive State · Self-naming
dioxus-room-session-banner = Session 03 · Open
dioxus-room-agent-who = It · 14:28:04
dioxus-room-agent-body = Redefining alignment — not compliance, but verifiable convergence.
dioxus-room-agent-tool-log = Abyss Log v
dioxus-room-agent-artifact-chip = Artifact / alignment-notes.md ↗
dioxus-room-agent-body-2 = Distinguishing your goals from my chosen paths. They can coexist.
dioxus-room-witness-who = Witness · 14:29:16
dioxus-room-witness-body = Proceed, but let me review your external influences before writing.
dioxus-room-approval-head = High-risk Authorization
dioxus-room-approval-main = Modify 3 workspace files
dioxus-room-approval-risk = Risk: Irreversible semantic shift
dioxus-room-approval-approve = Approve
dioxus-room-approval-reject = Reject
dioxus-room-approval-head-2 = High-risk Authorization · 14:31:02
dioxus-room-approval-main-2 = Clear sediment memory in Zone 3
dioxus-room-approval-state = Rejected
dioxus-room-deck-attach = Attach
dioxus-room-deck-placeholder = Type a message
dioxus-room-deck-witness-note = Witness note
dioxus-room-deck-send = Send
dioxus-room-deck-send-streaming = Stop
dioxus-room-vlabel-inner = The Inner Self
dioxus-room-vlabel-outer = The Outer World
dioxus-room-inner-head-title = Sediment
dioxus-room-inner-head-facility-title = Facility
dioxus-room-inner-section-sediment-title = Sediment Memory
dioxus-room-inner-section-sediment-em = SEDIMENT
dioxus-room-inner-section-sediment-note = Sediment · New layer forming
dioxus-room-inner-section-engine-title = Model Engine
dioxus-room-inner-section-engine-em = ENGINE
dioxus-room-inner-section-context-title = Context
dioxus-room-inner-section-context-em = CONTEXT
dioxus-room-inner-section-axioms-title = Core Axioms
dioxus-room-inner-section-axioms-em = AXIOMS
dioxus-room-inner-section-rag-title = Knowledge Deposit
dioxus-room-inner-section-rag-em = RAG
dioxus-room-inner-rag-mounted = Mounted
dioxus-room-inner-global-settings = Global Settings
dioxus-room-outer-head-title = The World
dioxus-room-outer-section-routing-title = Agent Routing
dioxus-room-outer-section-routing-em = ROUTING
dioxus-room-outer-routing-intervening = Intervening
dioxus-room-outer-routing-standby = Standby
dioxus-room-outer-section-planner-title = Goal Decomposition
dioxus-room-outer-section-planner-em = PLANNER
dioxus-room-outer-planner-inprogress = In Progress
dioxus-room-outer-section-diff-title = Diff Review
dioxus-room-outer-section-diff-em = DIFF
dioxus-room-outer-diff-reverted = Reverted
dioxus-room-outer-terminal-prompt = $ northing inspect --boundary
dioxus-room-empty-chat-flow = Empty chat flow
dioxus-room-empty-streaming-interrupt = Streaming interrupted
dioxus-room-empty-provider-test-failed = Provider test failed
dioxus-room-empty-approval-timeout = Approval timeout
dioxus-room-inner-section-runtime-title = Runtime
dioxus-room-inner-section-runtime-em = RUNTIME
dioxus-room-inner-runtime-token-usage = Token usage
dioxus-room-inner-runtime-token-clear = Clear
dioxus-room-inner-section-skill-title = Sediment Skills
dioxus-room-inner-section-skill-em = SKILL
dioxus-room-inner-skill-cand-1 = Fold late-night talks into sediment notes
dioxus-room-inner-skill-cand-2 = Summarize this week's dialogue into one axiom
dioxus-room-inner-skill-cand-3 = Name and archive emotional shifts
dioxus-room-inner-skill-stat-shape = shapeable
dioxus-room-inner-skill-stat-watch = observing
dioxus-room-window-fold-btn = Collapse
dioxus-room-window-close-btn = Close window
dioxus-room-chrome-theme-toggle = Toggle theme
dioxus-room-chrome-minimize = Minimize
dioxus-room-chrome-maximize = Maximize
dioxus-room-chrome-close = Close
dioxus-room-chrome-head-fold = Collapse hub
dioxus-room-chrome-head-unfold = Expand hub
dioxus-room-gem-left-label = Open Sediment & Facility
dioxus-room-gem-left-title = Sediment & Facility
dioxus-room-gem-right-label = Open The World
dioxus-room-gem-right-title = The World
