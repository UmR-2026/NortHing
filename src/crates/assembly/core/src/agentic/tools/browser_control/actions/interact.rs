use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

impl super::BrowserActions<'_> {
    pub async fn click(&self, selector: &str) -> NortHingResult<Value> {
        let (x, y) = self.element_center(selector).await?;

        self.client
            .send(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": "mousePressed",
                    "x": x, "y": y,
                    "button": "left", "clickCount": 1
                })),
            )
            .await?;
        self.client
            .send(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": "mouseReleased",
                    "x": x, "y": y,
                    "button": "left", "clickCount": 1
                })),
            )
            .await?;

        Ok(json!({
            "success": true,
            "action": "click",
            "selector": selector,
            "coordinates": { "x": x, "y": y }
        }))
    }

    async fn element_center(&self, selector: &str) -> NortHingResult<(f64, f64)> {
        let js = super::resolve_element_js(selector);
        let center_js = format!(
            r#"(function(){{ {} el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }}); const rect = el.getBoundingClientRect(); return JSON.stringify({{ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }}); }})()"#,
            js
        );
        let result = self.evaluate(&center_js).await?;
        let coords_str = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        let coords: Value = serde_json::from_str(coords_str).unwrap_or(json!({}));
        let x = coords.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = coords.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
        Ok((x, y))
    }

    pub async fn hover(&self, selector: &str) -> NortHingResult<Value> {
        let (x, y) = self.element_center(selector).await?;
        self.client
            .send(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": "mouseMoved",
                    "x": x, "y": y,
                    "button": "none"
                })),
            )
            .await?;
        Ok(json!({
            "success": true,
            "action": "hover",
            "selector": selector,
            "coordinates": { "x": x, "y": y }
        }))
    }

    /// Fill (clear + type) a text input identified by selector or `@eN` ref.
    pub async fn fill(&self, selector: &str, value: &str) -> NortHingResult<Value> {
        let js = super::resolve_element_js(selector);
        let focus_js = format!(
            r#"(function(){{ {} el.focus(); el.value = ''; el.dispatchEvent(new Event('input', {{ bubbles: true }})); return true; }})()"#,
            js
        );
        self.evaluate(&focus_js).await?;

        self.client
            .send("Input.insertText", Some(json!({ "text": value })))
            .await?;

        Ok(json!({
            "success": true,
            "action": "fill",
            "selector": selector,
        }))
    }

    /// Type text at the currently focused element (appends, does not clear).
    pub async fn type_text(&self, text: &str) -> NortHingResult<Value> {
        self.client
            .send("Input.insertText", Some(json!({ "text": text })))
            .await?;
        Ok(json!({ "success": true, "action": "type", "text": text }))
    }

    pub async fn set_checked(&self, selector: &str, checked: bool) -> NortHingResult<Value> {
        let js = super::resolve_element_js(selector);
        let script = format!(
            r#"(function(){{
                {js}
                el.checked = {checked};
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return JSON.stringify({{ success: true, checked: !!el.checked }});
            }})()"#,
            js = js,
            checked = if checked { "true" } else { "false" }
        );
        let result = self.evaluate(&script).await?;
        let text = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        let parsed: Value = serde_json::from_str(text).unwrap_or(json!({}));
        Ok(json!({
            "success": parsed.get("success").and_then(|v| v.as_bool()).unwrap_or(true),
            "action": if checked { "check" } else { "uncheck" },
            "selector": selector,
            "checked": parsed.get("checked").cloned().unwrap_or(json!(checked)),
        }))
    }

    /// Select a dropdown option by visible text.
    pub async fn select(&self, selector: &str, option_text: &str) -> NortHingResult<Value> {
        let js = format!(
            r#"(function(){{
                const sel = document.querySelector('{}');
                if (!sel) return JSON.stringify({{ error: 'Select not found' }});
                const opts = Array.from(sel.options);
                const opt = opts.find(o => o.text.includes('{}'));
                if (!opt) return JSON.stringify({{ error: 'Option not found', available: opts.map(o => o.text) }});
                sel.value = opt.value;
                sel.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return JSON.stringify({{ success: true, value: opt.value, text: opt.text }});
            }})()"#,
            selector.replace('\'', "\\'"),
            option_text.replace('\'', "\\'")
        );
        let result = self.evaluate(&js).await?;
        let text = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        let parsed: Value = serde_json::from_str(text).unwrap_or(json!({}));
        Ok(parsed)
    }

    /// Press a key (Enter, Escape, Tab, etc.).
    pub async fn press_key(&self, key: &str) -> NortHingResult<Value> {
        self.client
            .send(
                "Input.dispatchKeyEvent",
                Some(json!({
                    "type": "keyDown",
                    "key": key,
                })),
            )
            .await?;
        if key.chars().count() == 1 {
            self.client
                .send(
                    "Input.dispatchKeyEvent",
                    Some(json!({
                        "type": "char",
                        "key": key,
                        "text": key,
                    })),
                )
                .await?;
        }
        self.client
            .send(
                "Input.dispatchKeyEvent",
                Some(json!({
                    "type": "keyUp",
                    "key": key,
                })),
            )
            .await?;
        Ok(json!({ "success": true, "action": "press_key", "key": key }))
    }

    /// Scroll the page.
    pub async fn scroll(&self, direction: &str, amount: Option<i64>) -> NortHingResult<Value> {
        let px = amount.unwrap_or(500);
        let (delta_x, delta_y) = match direction {
            "up" => (0, -px),
            "down" => (0, px),
            "left" => (-px, 0),
            "right" => (px, 0),
            "top" => {
                self.evaluate("window.scrollTo(0, 0)").await?;
                return Ok(json!({ "success": true, "action": "scroll", "direction": "top" }));
            }
            "bottom" => {
                self.evaluate("window.scrollTo(0, document.body.scrollHeight)").await?;
                return Ok(json!({ "success": true, "action": "scroll", "direction": "bottom" }));
            }
            _ => (0, px),
        };
        self.client
            .send(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": "mouseWheel",
                    "x": 400, "y": 300,
                    "deltaX": delta_x, "deltaY": delta_y,
                })),
            )
            .await?;
        Ok(json!({ "success": true, "action": "scroll", "direction": direction, "amount": px }))
    }

    pub async fn auto_scroll(&self, direction: &str, max_scrolls: u64, delay_ms: u64) -> NortHingResult<Value> {
        let max_scrolls = max_scrolls.clamp(1, 200);
        let delay_ms = delay_ms.clamp(0, 5_000);
        let delta = if direction == "up" {
            "-window.innerHeight"
        } else {
            "window.innerHeight"
        };
        let boundary = if direction == "up" {
            "window.scrollY === 0"
        } else {
            "window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 2"
        };
        let script = format!(
            r#"(async () => {{
                let scrolls = 0;
                while (scrolls < {max_scrolls}) {{
                    const before = window.scrollY;
                    window.scrollBy(0, {delta});
                    await new Promise(resolve => setTimeout(resolve, {delay_ms}));
                    scrolls++;
                    if ({boundary} || window.scrollY === before) break;
                }}
                return {{ scrolls, scrollY: window.scrollY, height: document.documentElement.scrollHeight }};
            }})()"#
        );
        let result = self.evaluate(&script).await?;
        Ok(json!({
            "success": true,
            "action": "auto_scroll",
            "direction": direction,
            "result": result.get("result").and_then(|r| r.get("value")).cloned().unwrap_or(Value::Null),
        }))
    }

    /// Wait for a duration or a condition.
    pub async fn wait(&self, duration_ms: Option<u64>, condition: Option<&str>) -> NortHingResult<Value> {
        if let Some(ms) = duration_ms {
            let clamped = ms.min(30_000);
            tokio::time::sleep(std::time::Duration::from_millis(clamped)).await;
            return Ok(json!({ "success": true, "action": "wait", "ms": clamped }));
        }
        if let Some(cond) = condition {
            match cond {
                "networkidle" | "load" | "domcontentloaded" => {
                    // Phase 1: replace the previous "sleep 2s and hope" with
                    // a real `Page.lifecycleEvent` subscription. Lifecycle
                    // event names per CDP: `load`, `DOMContentLoaded`,
                    // `networkIdle`, `firstMeaningfulPaint`, etc.
                    let _ = self.client.send("Page.enable", None).await;
                    let _ = self
                        .client
                        .send("Page.setLifecycleEventsEnabled", Some(json!({ "enabled": true })))
                        .await;
                    let mut events = self.client.subscribe_events();
                    let wanted: &[&str] = match cond {
                        "networkidle" => &["networkIdle"],
                        "domcontentloaded" => &["DOMContentLoaded", "load"],
                        _ => &["load"],
                    };
                    let outcome = super::wait_for_lifecycle(&mut events, None, wanted, 15_000).await;
                    let (success, lifecycle_event, timed_out) = match outcome {
                        super::LifecycleOutcome::Reached(n) => (true, Some(n), false),
                        super::LifecycleOutcome::Timeout => (false, None, true),
                        super::LifecycleOutcome::Closed => (false, None, false),
                    };
                    return Ok(json!({
                        "success": success,
                        "action": "wait",
                        "condition": cond,
                        "lifecycle_event": lifecycle_event,
                        "timed_out": timed_out,
                    }));
                }
                selector => {
                    for _ in 0..30 {
                        let js = format!("!!document.querySelector('{}')", selector.replace('\'', "\\'"));
                        let result = self.evaluate(&js).await?;
                        let found = result
                            .get("result")
                            .and_then(|r| r.get("value"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if found {
                            return Ok(json!({ "success": true, "action": "wait", "condition": cond }));
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                    return Err(NortHingError::tool(format!("Timeout waiting for element: {}", cond)));
                }
            }
        }
        Ok(json!({ "success": true, "action": "wait" }))
    }
}
