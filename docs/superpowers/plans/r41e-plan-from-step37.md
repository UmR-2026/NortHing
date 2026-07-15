| sibling filename          | sub-domain                          | line range (estimate) | 1-line description                                                                 |
|---------------------------|-------------------------------------|-----------------------|-----------------------------------------------------------------------------------|
| tool_context_runtime.rs   | Core ToolUseContext Definition      | 1-350                 | Defines the core ToolUseContext struct, its fields, base accessors, and portable context provider implementation. |
| tool_context_runtime.rs   | Runtime Execution Hooks             | 350-420               | Implements async tool call wrappers with cancellation support and post-call hook recording. |
| tool_context_runtime.rs   | Context Builder Functions           | 420-580               | Provides constructors for ToolUseContext from task, execution context, and listing inputs, plus custom data population logic. |
| tool_context_runtime.rs   | Path & Restriction Enforcement      | 580-720               | Implements path resolution, runtime restriction checks, and workspace path containment validation for tools. |
| tool_context_runtime.rs   | Checkpoint & Runtime Root Management| 720-900               | Handles workspace runtime root resolution, runtime context provisioning, and light checkpoint recording with git diff hashing. |
| tool_context_runtime.rs   | Unit Tests                          | 900-1447              | Contains test fixtures and assertions validating context materialization, field preservation, and fact projection correctness. |

| Item Type               | Item Name                          | Description                                                                 |
|-------------------------|------------------------------------|-----------------------------------------------------------------------------|
| Sibling Module Declaration | computer_use_host | Declares the computer use host tool module for desktop automation integration. |
| Sibling Module Declaration | framework | Declares the core tool framework module with path resolution and tool trait definitions. |
| Sibling Module Declaration | pipeline | Declares the tool execution pipeline module handling task and execution context types. |
| Sibling Module Declaration | post_call_hooks | Declares the post-call hooks module for recording successful tool execution events. |
| Sibling Module Declaration | restrictions | Declares the tool path restriction module for local and remote path containment checks. |
| Sibling Module Declaration | workspace_paths | Declares the workspace path utility module for runtime URI handling and path normalization. |
| Sibling Module Declaration | tool_context_runtime | Declares the current tool context runtime module. |
| Re-export               | ToolUseContext | Re-exports the core ToolUseContext struct for use across the agentic tool system. |