// Reserved for OpenAI error mapping, rate-limit handling, and retry logic.
// Currently stream errors are propagated as `anyhow::Error` / `String` at the
// call sites; this file preserves the split boundary for future extraction.
