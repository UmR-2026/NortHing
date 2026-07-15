use super::cdp_client::{CdpClient, CdpEvent};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tokio::sync::broadcast;

/// Result of waiting for a CDP `Page.lifecycleEvent`.
enum LifecycleOutcome {
    /// One of the requested lifecycle names fired in time. Carries the name
    /// (e.g. `"load"`, `"networkIdle"`) so callers can report which condition
    /// actually matched.
    Reached(String),
    /// Timed out before any of the requested events fired.
    Timeout,
    /// Subscription closed (typically: page navigated away or browser quit).
    Closed,
}

/// Block until a `Page.lifecycleEvent` whose `name` ∈ `wanted` arrives for the
/// given `frame_id` (or any frame if `frame_id` is `None`). Bounded by a hard
/// timeout so a hung page can never wedge the agent.
async fn wait_for_lifecycle(
    events: &mut broadcast::Receiver<CdpEvent>,
    frame_id: Option<&str>,
    wanted: &[&str],
    timeout_ms: u64,
) -> LifecycleOutcome {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return LifecycleOutcome::Timeout;
        }
        let recv_fut = events.recv();
        let evt = match tokio::time::timeout(remaining, recv_fut).await {
            Err(_) => return LifecycleOutcome::Timeout,
            Ok(Err(broadcast::error::RecvError::Closed)) => return LifecycleOutcome::Closed,
            // We deliberately swallow Lagged: lifecycle bursts can outpace
            // our buffer briefly; the next iteration will catch the next one.
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Ok(evt)) => evt,
        };
        if evt.method != "Page.lifecycleEvent" {
            continue;
        }
        let name = evt.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if !wanted.contains(&name) {
            continue;
        }
        if let Some(want_frame) = frame_id {
            let evt_frame = evt.params.get("frameId").and_then(|v| v.as_str()).unwrap_or("");
            if evt_frame != want_frame {
                continue;
            }
        }
        return LifecycleOutcome::Reached(name.to_string());
    }
}

/// High-level browser actions backed by CDP method calls.
pub struct BrowserActions<'a> {
    client: &'a CdpClient,
}

impl<'a> BrowserActions<'a> {
    pub fn new(client: &'a CdpClient) -> Self {
        Self { client }
    }
}

/// Generate JS to resolve an element from selector or `@eN` ref.
///
/// Phase 3: ref / selector lookup walks open shadow roots and
/// same-origin iframes so refs / selectors created by `snapshot()` for
/// elements inside a shadow root or iframe actually resolve. The legacy
/// `document.querySelector` path returned `null` for any element nested
/// inside a shadow root, which made `click @e7` mysteriously fail
/// whenever the page used a web-component design system.
fn resolve_element_js(selector: &str) -> String {
    let attr_selector = if selector.starts_with("@e") {
        format!(r#"[data-cdp-ref="{}"]"#, selector)
    } else {
        selector.to_string()
    };
    let escaped = attr_selector.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"
        const __sel = '{escaped}';
        function __findIn(root) {{
            try {{
                const direct = root.querySelector(__sel);
                if (direct) return direct;
            }} catch (_) {{}}
            const all = root.querySelectorAll('*');
            for (const node of all) {{
                if (node.shadowRoot) {{
                    const hit = __findIn(node.shadowRoot);
                    if (hit) return hit;
                }}
            }}
            return null;
        }}
        function __findAnywhere() {{
            const top = __findIn(document);
            if (top) return top;
            const frames = document.querySelectorAll('iframe, frame');
            for (const f of frames) {{
                let doc = null;
                try {{ doc = f.contentDocument; }} catch (_) {{}}
                if (doc) {{
                    const hit = __findIn(doc);
                    if (hit) return hit;
                }}
            }}
            return null;
        }}
        const el = __findAnywhere();
        if (!el) throw new Error('Element not found: ' + __sel + ' — take a fresh snapshot or check shadow/iframe scope');
        "#,
        escaped = escaped
    )
}

mod extract;
mod format;
mod interact;
mod navigation;
