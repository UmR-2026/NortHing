use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

impl super::BrowserActions<'_> {
    pub async fn enable_observers(&self) -> NortHingResult<Value> {
        let _ = self.client.send("Page.enable", None).await;
        let _ = self.client.send("Runtime.enable", None).await;
        let _ = self.client.send("Network.enable", None).await;
        let _ = self.client.send("DOM.enable", None).await;
        Ok(json!({ "success": true, "action": "enable_observers" }))
    }

    // ── Navigation ─────────────────────────────────────────────────────

    pub async fn navigate(&self, url: &str) -> NortHingResult<Value> {
        // Subscribe **before** issuing the navigate so we can never miss the
        // `Page.lifecycleEvent` ("load") that fires while we are awaiting the
        // command response. Page lifecycle events must be enabled explicitly.
        let _ = self.client.send("Page.enable", None).await;
        let _ = self
            .client
            .send("Page.setLifecycleEventsEnabled", Some(json!({ "enabled": true })))
            .await;
        let mut events = self.client.subscribe_events();

        let result = self.client.send("Page.navigate", Some(json!({ "url": url }))).await?;
        let frame_id = result.get("frameId").and_then(|v| v.as_str()).map(str::to_string);

        // Wait for the matching "load" lifecycle event (or "DOMContentLoaded"
        // as an early signal). Capped at ~15s so a hung page eventually
        // surfaces a Timeout error to the model rather than blocking forever.
        let outcome = super::wait_for_lifecycle(&mut events, frame_id.as_deref(), &["load"], 15_000).await;

        let mut body = json!({
            "url": url,
            "frameId": frame_id,
        });
        match outcome {
            super::LifecycleOutcome::Reached(name) => {
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("success".to_string(), json!(true));
                    obj.insert("loaded".to_string(), json!(true));
                    obj.insert("lifecycle_event".to_string(), json!(name));
                }
            }
            super::LifecycleOutcome::Timeout => {
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("success".to_string(), json!(true));
                    obj.insert("loaded".to_string(), json!(false));
                    obj.insert(
                        "warning".to_string(),
                        json!("navigation timed out before lifecycle 'load' event; page may still be loading"),
                    );
                }
            }
            super::LifecycleOutcome::Closed => {
                return Err(NortHingError::tool(
                    "Browser closed the CDP connection before page finished loading.".to_string(),
                ));
            }
        }
        Ok(body)
    }

    pub async fn back(&self) -> NortHingResult<Value> {
        self.evaluate("history.back(); undefined").await?;
        Ok(json!({ "success": true, "action": "back" }))
    }

    pub async fn forward(&self) -> NortHingResult<Value> {
        self.evaluate("history.forward(); undefined").await?;
        Ok(json!({ "success": true, "action": "forward" }))
    }

    pub async fn reload(&self, ignore_cache: bool) -> NortHingResult<Value> {
        self.client
            .send("Page.reload", Some(json!({ "ignoreCache": ignore_cache })))
            .await?;
        Ok(json!({ "success": true, "action": "reload", "ignore_cache": ignore_cache }))
    }

    pub async fn get_url(&self) -> NortHingResult<String> {
        let result = self.evaluate("window.location.href").await?;
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    pub async fn get_title(&self) -> NortHingResult<String> {
        let result = self.evaluate("document.title").await?;
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    // ── Snapshot / DOM ─────────────────────────────────────────────────

    /// Get an accessibility-tree snapshot of interactive elements.
    ///
    /// Phase 3: traversal now descends into **open shadow roots** and
    /// **same-origin iframes**, which the old flat `document.querySelectorAll`
    /// path silently skipped. Each element's `frame_path` reports where in
    /// the frame tree it lives (`""` for top frame,
    /// `"iframe[src='/foo']"` for an iframe child) and its `scope` reports
    /// `"document" | "shadow" | "iframe"`. The synthetic `data-cdp-ref`
    /// attribute is set in the host scope so subsequent `click` / `fill`
    /// can locate it via the same recursive walk.
    pub async fn close_page(&self) -> NortHingResult<Value> {
        let _ = self.client.send("Page.close", None).await;
        Ok(json!({ "success": true, "action": "close" }))
    }
}
