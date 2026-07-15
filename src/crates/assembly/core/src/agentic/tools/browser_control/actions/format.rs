use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

impl super::BrowserActions<'_> {
    /// Evaluate a JavaScript expression in the page context.
    pub async fn evaluate(&self, expression: &str) -> NortHingResult<Value> {
        self.evaluate_with_options(expression, true, true).await
    }

    pub async fn evaluate_with_options(
        &self,
        expression: &str,
        await_promise: bool,
        return_by_value: bool,
    ) -> NortHingResult<Value> {
        let mut last_error: Option<NortHingError> = None;
        for attempt in 0..2 {
            let result = self
                .client
                .send(
                    "Runtime.evaluate",
                    Some(json!({
                        "expression": expression,
                        "returnByValue": return_by_value,
                        "awaitPromise": await_promise,
                    })),
                )
                .await;
            match result {
                Ok(value) => {
                    if let Some(details) = value.get("exceptionDetails") {
                        let message = details
                            .get("exception")
                            .and_then(|e| e.get("description"))
                            .and_then(|v| v.as_str())
                            .or_else(|| details.get("text").and_then(|v| v.as_str()))
                            .unwrap_or("Runtime.evaluate failed");
                        return Err(NortHingError::tool(format!("JS error: {}", message)));
                    }
                    return Ok(value);
                }
                Err(err) => {
                    let message = err.to_string();
                    let retryable = message.contains("Inspected target navigated")
                        || message.contains("Target closed")
                        || message.contains("Cannot find context with specified id");
                    last_error = Some(err);
                    if retryable && attempt == 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        continue;
                    }
                    break;
                }
            }
        }
        Err(last_error.unwrap_or_else(|| NortHingError::tool("Runtime.evaluate failed".to_string())))
    }

    pub async fn get_cookies(&self, urls: Option<Vec<String>>) -> NortHingResult<Value> {
        let params = urls
            .filter(|items| !items.is_empty())
            .map(|urls| json!({ "urls": urls }))
            .unwrap_or_else(|| json!({}));
        let result = self.client.send("Network.getCookies", Some(params)).await?;
        Ok(json!({
            "success": true,
            "action": "cookies",
            "cookies": result.get("cookies").cloned().unwrap_or_else(|| json!([])),
        }))
    }

    pub async fn set_cookies(&self, cookies: &[Value]) -> NortHingResult<Value> {
        let mut set = 0usize;
        let mut errors = Vec::<Value>::new();
        for cookie in cookies {
            match self.client.send("Network.setCookie", Some(cookie.clone())).await {
                Ok(result) if result.get("success").and_then(|v| v.as_bool()).unwrap_or(true) => {
                    set += 1;
                }
                Ok(result) => errors.push(json!({ "cookie": cookie, "result": result })),
                Err(err) => errors.push(json!({ "cookie": cookie, "error": err.to_string() })),
            }
        }
        Ok(json!({
            "success": errors.is_empty(),
            "action": "set_cookies",
            "set": set,
            "errors": errors,
        }))
    }

    pub async fn set_file_input_files(&self, selector: Option<&str>, files: &[String]) -> NortHingResult<Value> {
        if files.is_empty() {
            return Err(NortHingError::tool(
                "set_file_input_files requires non-empty 'files'".to_string(),
            ));
        }
        let query = selector.unwrap_or("input[type=\"file\"]");
        let css_selector = if query.starts_with("@e") {
            format!(r#"[data-cdp-ref="{}"]"#, query)
        } else {
            query.to_string()
        };
        let document = self.client.send("DOM.getDocument", None).await?;
        let root_id = document
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| NortHingError::tool("DOM.getDocument: missing root nodeId".to_string()))?;
        let node = self
            .client
            .send(
                "DOM.querySelector",
                Some(json!({ "nodeId": root_id, "selector": css_selector })),
            )
            .await?;
        let node_id = node.get("nodeId").and_then(|v| v.as_i64()).unwrap_or(0);
        if node_id == 0 {
            return Err(NortHingError::tool(format!(
                "No file input found for selector: {}",
                query
            )));
        }
        self.client
            .send(
                "DOM.setFileInputFiles",
                Some(json!({ "nodeId": node_id, "files": files })),
            )
            .await?;
        Ok(json!({
            "success": true,
            "action": "set_file_input_files",
            "selector": query,
            "count": files.len(),
        }))
    }

    pub async fn fetch(&self, url: &str, method: &str, headers: Value, body: Option<&str>) -> NortHingResult<Value> {
        let script = format!(
            r#"(async () => {{
                try {{
                    const init = {{
                        method: {method},
                        credentials: 'include',
                        headers: {headers}
                    }};
                    const body = {body};
                    if (body !== null && init.method !== 'GET' && init.method !== 'HEAD') init.body = body;
                    const resp = await fetch({url}, init);
                    const contentType = resp.headers.get('content-type') || '';
                    let responseBody;
                    if (contentType.includes('application/json') && resp.status !== 204) {{
                        try {{ responseBody = await resp.json(); }} catch (_) {{ responseBody = await resp.text(); }}
                    }} else {{
                        responseBody = await resp.text();
                    }}
                    return JSON.stringify({{
                        ok: resp.ok,
                        status: resp.status,
                        status_text: resp.statusText,
                        content_type: contentType,
                        url: resp.url,
                        body: responseBody
                    }});
                }} catch (e) {{
                    return JSON.stringify({{ error: String(e && e.message || e) }});
                }}
            }})()"#,
            url = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".to_string()),
            method = serde_json::to_string(&method.to_uppercase()).unwrap_or_else(|_| "\"GET\"".to_string()),
            headers = headers,
            body = body
                .map(|b| serde_json::to_string(b).unwrap_or_else(|_| "null".to_string()))
                .unwrap_or_else(|| "null".to_string()),
        );
        let result = self.evaluate(&script).await?;
        let raw = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        let parsed: Value = serde_json::from_str(raw).unwrap_or(json!({}));
        Ok(json!({ "success": parsed.get("error").is_none(), "action": "fetch", "result": parsed }))
    }

    pub async fn read_article(&self) -> NortHingResult<Value> {
        let script = r#"
        (function() {
            function textOf(node) {
                return (node && node.textContent || '').replace(/\s+/g, ' ').trim();
            }
            const article = document.querySelector('article') || document.querySelector('main') || document.body;
            const title = document.querySelector('meta[property="og:title"]')?.content || document.title || '';
            const description = document.querySelector('meta[name="description"]')?.content || document.querySelector('meta[property="og:description"]')?.content || '';
            const siteName = document.querySelector('meta[property="og:site_name"]')?.content || location.hostname;
            const publishedTime = document.querySelector('meta[property="article:published_time"]')?.content || document.querySelector('time[datetime]')?.getAttribute('datetime') || null;
            const textContent = textOf(article);
            const headings = Array.from(article.querySelectorAll('h1,h2,h3')).slice(0, 20).map(h => ({ level: h.tagName.toLowerCase(), text: textOf(h) })).filter(h => h.text);
            return {
                title,
                description,
                siteName,
                publishedTime,
                url: location.href,
                length: textContent.length,
                excerpt: textContent.slice(0, 500),
                textContent,
                headings,
            };
        })()
        "#;
        let result = self.evaluate(script).await?;
        Ok(json!({
            "success": true,
            "action": "read_article",
            "article": result.get("result").and_then(|r| r.get("value")).cloned().unwrap_or(Value::Null),
        }))
    }

    // ── Close ──────────────────────────────────────────────────────────
}
