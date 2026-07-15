| sibling filename | sub-domain | line range (estimate) | 1-line description |
|------------------|------------|----------------------|--------------------|
| coordination.rs | agentic::coordination | 5 | Provides access to the global coordinator |
| core.rs | agentic::core | 6‑440 | Defines `CompressionContract` for deep review |
| report.rs | agentic::deep_review::report | 7‑800 | Deep review utilities: context detection, metadata, signals, cache |
| framework.rs | agentic::tools::framework | 8‑612 | `Tool` trait, `ToolResult` type, and `ToolUseContext` |
| config.rs | service::config | 9‑592 | Retrieves application language code |
| i18n.rs | service::i18n | 10‑48 | Localized strings for code review UI |
| errors.rs | util::errors | 11‑577 | `NortHingResult` error handling type |

| sibling module | re-export |
|----------------|-----------|
| code_review_tool | `pub use code_review_tool::CodeReviewTool;` |
| framework | `pub use framework::{Tool, ToolResult, ToolUseContext};` |