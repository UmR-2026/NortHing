use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};
use std::collections::BTreeMap;

impl super::BrowserActions<'_> {
    /// can locate it via the same recursive walk.
    pub async fn snapshot(&self) -> NortHingResult<Value> {
        self.snapshot_with_options(false).await
    }

    /// Snapshot variant that can additionally resolve a stable
    /// **backendNodeId** (CDP `DOM.Node.backendNodeId`) for each element.
    /// `backendNodeId` is invariant across reflows and JS re-renders within
    /// the same DOM lifetime, so saving it lets the agent re-target an
    /// element after a partial mutation without taking a full snapshot.
    ///
    /// The call is opt-in (and slightly more expensive) because it costs
    /// one extra CDP round-trip plus a `DOM.querySelectorAll` walk. When
    /// `with_backend_node_ids` is `true`, every snapshot element gets a
    /// `backend_node_id` field; pages where `DOM.getDocument` errors out
    /// (very rare — e.g. about:blank) silently fall back to no ids.
    pub async fn snapshot_with_options(&self, with_backend_node_ids: bool) -> NortHingResult<Value> {
        let script = r#"
        (function() {
            const SEL = 'a, button, input, textarea, select, [role="button"], [role="link"], [role="tab"], [role="menuitem"], [role="combobox"], [role="option"], [tabindex="0"], [contenteditable="true"]';
            const items = [];
            let idx = 1;

            function visible(el, win) {
                const rect = el.getBoundingClientRect();
                if (rect.width < 2 || rect.height < 2) return null;
                if (rect.right < 0 || rect.bottom < 0 || rect.left > win.innerWidth || rect.top > win.innerHeight) return null;
                const style = win.getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return null;
                return rect;
            }

            function record(el, rect, scope, framePath) {
                const text = (el.textContent || '').trim().slice(0, 100);
                items.push({
                    ref: '@e' + idx,
                    tag: el.tagName.toLowerCase(),
                    type: el.getAttribute('type') || '',
                    name: el.getAttribute('name') || '',
                    text,
                    ariaLabel: el.getAttribute('aria-label') || '',
                    placeholder: el.placeholder || '',
                    role: el.getAttribute('role') || '',
                    href: el.href || '',
                    id: el.id || '',
                    scope,
                    frame_path: framePath,
                    rect: { x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height) }
                });
                try { el.setAttribute('data-cdp-ref', '@e' + idx); } catch (_) {}
                idx++;
            }

            // Recursive walk: collects from `root` (Document or ShadowRoot)
            // and recurses into open shadow roots of every descendant. Iframes
            // are handled by the caller because we need the iframe's own
            // window for visibility checks.
            function walk(root, win, scope, framePath) {
                const els = root.querySelectorAll(SEL);
                els.forEach(el => {
                    const rect = visible(el, win);
                    if (rect) record(el, rect, scope, framePath);
                });
                // Open shadow roots
                const allHosts = root.querySelectorAll('*');
                allHosts.forEach(h => {
                    if (h.shadowRoot) {
                        try { walk(h.shadowRoot, win, 'shadow', framePath); } catch (_) {}
                    }
                });
            }

            walk(document, window, 'document', '');

            // Same-origin iframes
            const frames = document.querySelectorAll('iframe, frame');
            frames.forEach((frame, fi) => {
                let doc = null;
                try { doc = frame.contentDocument; } catch (_) {}
                if (!doc) return; // cross-origin: skip silently
                const subWin = frame.contentWindow;
                const path = `iframe[${fi}]${frame.src ? `[src="${frame.src.slice(0, 80)}"]` : ''}`;
                try { walk(doc, subWin, 'iframe', path); } catch (_) {}
            });

            return JSON.stringify({
                url: location.href,
                title: document.title,
                elements: items,
                features: { shadow_dom_traversed: true, same_origin_iframes_traversed: true },
            });
        })()
        "#;
        let result = self.evaluate(script).await?;
        let text = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        let mut parsed: Value = serde_json::from_str(text).unwrap_or(json!({}));

        if with_backend_node_ids {
            if let Err(e) = self.attach_backend_node_ids(&mut parsed).await {
                // Don't fail the snapshot �?the elements list is still
                // useful without backendNodeIds. Surface the failure so the
                // model can decide whether to retry.
                if let Value::Object(m) = &mut parsed {
                    m.insert(
                        "backend_node_ids_warning".to_string(),
                        json!(format!("Failed to resolve backendNodeIds: {}", e)),
                    );
                }
            }
        }
        Self::attach_snapshot_text(&mut parsed);
        Ok(parsed)
    }

    fn attach_snapshot_text(parsed: &mut Value) {
        let Some(elements) = parsed.get("elements").and_then(|v| v.as_array()) else {
            return;
        };
        let mut lines = Vec::<String>::new();
        let mut refs = BTreeMap::<String, Value>::new();
        for element in elements {
            let reference = element.get("ref").and_then(|v| v.as_str()).unwrap_or("");
            let tag = element.get("tag").and_then(|v| v.as_str()).unwrap_or("element");
            let role = element.get("role").and_then(|v| v.as_str()).unwrap_or("");
            let text = element
                .get("ariaLabel")
                .or_else(|| element.get("placeholder"))
                .or_else(|| element.get("text"))
                .or_else(|| element.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let type_text = element.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let id = element.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let frame_path = element.get("frame_path").and_then(|v| v.as_str()).unwrap_or("");
            let scope = element.get("scope").and_then(|v| v.as_str()).unwrap_or("document");
            let mut label = if role.is_empty() {
                tag.to_string()
            } else {
                role.to_string()
            };
            if !type_text.is_empty() {
                label.push(':');
                label.push_str(type_text);
            }
            let mut line = if reference.is_empty() {
                format!("- {}", label)
            } else {
                format!("- {} [{}]", label, reference)
            };
            if !text.is_empty() {
                let clipped = if text.chars().count() > 80 {
                    format!("{}...", text.chars().take(77).collect::<String>())
                } else {
                    text.to_string()
                };
                line.push(' ');
                line.push_str(&serde_json::to_string(&clipped).unwrap_or_else(|_| "\"\"".to_string()));
            }
            if !id.is_empty() {
                line.push_str(&format!(" id={}", id));
            }
            if scope != "document" || !frame_path.is_empty() {
                line.push_str(&format!(" scope={}", scope));
                if !frame_path.is_empty() {
                    line.push_str(&format!(" frame={}", frame_path));
                }
            }
            lines.push(line);
            if !reference.is_empty() {
                refs.insert(reference.to_string(), element.clone());
            }
        }
        if let Some(obj) = parsed.as_object_mut() {
            obj.insert("snapshot".to_string(), json!(lines.join("\n")));
            obj.insert("refs".to_string(), json!(refs));
        }
    }

    /// Resolve `backend_node_id` for every snapshot element by walking the
    /// DOM through CDP. Mutates `parsed["elements"][i]["backend_node_id"]`
    /// in place. Returns `Err` if the document tree could not be fetched.
    async fn attach_backend_node_ids(&self, parsed: &mut Value) -> NortHingResult<()> {
        let doc = self.client.send("DOM.getDocument", None).await?;
        let root_id = doc
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| NortHingError::tool("DOM.getDocument: missing root nodeId".to_string()))?;
        let qsa = self
            .client
            .send(
                "DOM.querySelectorAll",
                Some(json!({ "nodeId": root_id, "selector": "[data-cdp-ref]" })),
            )
            .await?;
        let node_ids: Vec<i64> = qsa
            .get("nodeIds")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|n| n.as_i64()).collect())
            .unwrap_or_default();

        let mut by_ref: std::collections::HashMap<String, i64> = Default::default();
        for nid in node_ids {
            let described = match self
                .client
                .send("DOM.describeNode", Some(json!({ "nodeId": nid })))
                .await
            {
                Ok(d) => d,
                Err(_) => continue,
            };
            let backend = described
                .get("node")
                .and_then(|n| n.get("backendNodeId"))
                .and_then(|v| v.as_i64());
            // Read the data-cdp-ref attribute from the node's attributes
            // (DOM.describeNode returns flat [name, value, name, value]).
            let attrs = described
                .get("node")
                .and_then(|n| n.get("attributes"))
                .and_then(|v| v.as_array());
            let ref_name = attrs.and_then(|a| {
                a.chunks(2)
                    .find(|c| c.first().and_then(|n| n.as_str()) == Some("data-cdp-ref"))
                    .and_then(|c| c.get(1).and_then(|v| v.as_str().map(str::to_string)))
            });
            if let (Some(rn), Some(b)) = (ref_name, backend) {
                by_ref.insert(rn, b);
            }
        }

        if let Some(elements) = parsed.get_mut("elements").and_then(|v| v.as_array_mut()) {
            for el in elements.iter_mut() {
                let r = el.get("ref").and_then(|v| v.as_str()).map(str::to_string);
                if let Some(r) = r {
                    if let Some(b) = by_ref.get(&r) {
                        if let Value::Object(m) = el {
                            m.insert("backend_node_id".to_string(), json!(b));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get the text content of an element by CSS selector or `@eN` ref.
    ///
    /// Phase 3: returns `Ok(None)` when the selector matched nothing (so
    /// ControlHub can surface a `NOT_FOUND` error instead of a misleading
    /// empty string), and `Ok(Some(""))` when the element was found but
    /// genuinely empty. The lookup walks shadow roots / same-origin
    /// iframes, matching the rest of the browser action surface.
    pub async fn get_text(&self, selector: &str) -> NortHingResult<Option<String>> {
        self.get_attribute(selector, "text")
            .await
            .map(|v| v.map(|v| v.as_str().unwrap_or("").to_string()))
    }

    pub async fn get_attribute(&self, selector: &str, attribute: &str) -> NortHingResult<Option<Value>> {
        let resolve = super::resolve_element_js(selector);
        let getter = match attribute {
            "text" => "(el.textContent || '').trim().slice(0, 5000)".to_string(),
            "value" => "('value' in el ? el.value : '')".to_string(),
            "html" => "el.outerHTML".to_string(),
            other => format!(
                "el.getAttribute('{}')",
                other.replace('\\', "\\\\").replace('\'', "\\'")
            ),
        };
        let js = format!(
            r#"(function(){{
                try {{
                    {resolve}
                    return JSON.stringify({{ found: true, value: {getter} }});
                }} catch (e) {{
                    return JSON.stringify({{ found: false, error: String(e && e.message || e) }});
                }}
            }})()"#,
            resolve = resolve,
            getter = getter,
        );
        let result = self.evaluate(&js).await?;
        let raw = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        let parsed: Value = serde_json::from_str(raw).unwrap_or(json!({}));
        if parsed.get("found").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(Some(parsed.get("value").cloned().unwrap_or(Value::Null)))
        } else {
            Ok(None)
        }
    }

    // ── Interaction ────────────────────────────────────────────────────

    /// Click an element by CSS selector or by `@eN` ref.
    pub async fn screenshot(&self) -> NortHingResult<Value> {
        self.screenshot_with_options("jpeg", Some(80), true).await
    }

    pub async fn screenshot_with_options(
        &self,
        format: &str,
        quality: Option<u8>,
        from_surface: bool,
    ) -> NortHingResult<Value> {
        self.screenshot_with_options_ext(format, quality, from_surface, false)
            .await
    }

    pub async fn screenshot_with_options_ext(
        &self,
        format: &str,
        quality: Option<u8>,
        from_surface: bool,
        full_page: bool,
    ) -> NortHingResult<Value> {
        let normalized = if format.eq_ignore_ascii_case("png") {
            "png"
        } else {
            "jpeg"
        };

        if full_page {
            if let Ok(metrics) = self.client.send("Page.getLayoutMetrics", None).await {
                let size = metrics.get("cssContentSize").or_else(|| metrics.get("contentSize"));
                if let Some(size) = size {
                    let width = size.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0).ceil() as u64;
                    let height = size.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0).ceil() as u64;
                    if width > 0 && height > 0 {
                        let _ = self
                            .client
                            .send(
                                "Emulation.setDeviceMetricsOverride",
                                Some(json!({
                                    "mobile": false,
                                    "width": width,
                                    "height": height,
                                    "deviceScaleFactor": 1,
                                })),
                            )
                            .await;
                    }
                }
            }
        }

        let mut params = json!({
            "format": normalized,
            "fromSurface": from_surface,
        });
        if normalized == "jpeg" {
            params["quality"] = json!(quality.unwrap_or(80).min(100));
        }
        let result = self.client.send("Page.captureScreenshot", Some(params)).await?;
        if full_page {
            let _ = self.client.send("Emulation.clearDeviceMetricsOverride", None).await;
        }
        let data = result.get("data").and_then(|v| v.as_str()).unwrap_or("");
        Ok(json!({
            "success": true,
            "action": "screenshot",
            "format": normalized,
            "full_page": full_page,
            "data_length": data.len(),
            "base64_data": data,
            "data_url": format!("data:image/{};base64,{}", normalized, data),
        }))
    }
}
