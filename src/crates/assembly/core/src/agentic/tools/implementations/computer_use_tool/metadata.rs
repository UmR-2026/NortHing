//! ComputerUse tool metadata: tool descriptions, JSON schemas, host-OS labels,
//! and the shared `computer_use_augment_result_json` helper used by every
//! action handler to inject session context + loop detection into tool output.
//!
//! These helpers are mostly pure (no `self` state) — they sit on `ComputerUseTool`
//! as inherent methods so `impl Tool for ComputerUseTool` (in `mod.rs`) can
//! delegate with `Self::method_name()`.

use crate::agentic::tools::computer_use_host::ComputerUseHost;
use crate::agentic::tools::framework::ToolUseContext;
use crate::util::errors::NortHingError;
use crate::util::errors::NortHingResult;
use serde_json::{json, Value};

use super::ComputerUseTool;

/// Merges [`ComputerUseHost::computer_use_session_snapshot`] + optional `input_coordinates` into tool JSON.
/// Also records the action for loop detection and adds loop warnings if detected.
pub(crate) async fn computer_use_augment_result_json(
    host: &dyn ComputerUseHost,
    mut body: Value,
    input_coordinates: Option<Value>,
) -> Value {
    let snap = host.computer_use_session_snapshot().await;
    let interaction = host.computer_use_interaction_state();

    // Record action for loop detection
    let action_type = body
        .get("action")
        .or_else(|| body.get("tool"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let action_params = input_coordinates.as_ref().map(|v| v.to_string()).unwrap_or_default();
    let success = body.get("success").and_then(|v| v.as_bool()).unwrap_or(true);
    host.record_action(&action_type, &action_params, success);

    // Check for action loops
    let loop_result = host.detect_action_loop();

    if let Value::Object(map) = &mut body {
        map.insert(
            "computer_use_context".to_string(),
            json!({
                "foreground_application": snap.foreground_application,
                "pointer_global": snap.pointer_global,
                "input_coordinates": input_coordinates,
            }),
        );
        map.insert("interaction_state".to_string(), json!(interaction));

        // Loop hint surfaced to the model as a warning only — it never forces the
        // agent loop to stop. The model decides on its own whether to switch tactic.
        if loop_result.is_loop {
            map.insert(
                "loop_warning".to_string(),
                json!({
                    "detected": true,
                    "pattern_length": loop_result.pattern_length,
                    "repetitions": loop_result.repetitions,
                    "suggestion": loop_result.suggestion,
                }),
            );
        }
    }
    body
}

impl ComputerUseTool {
    /// Tool description when the primary model is **text-only** (no `screenshot` / JPEG workflow).
    pub(crate) fn description_text_only_impl() -> String {
        let os = Self::host_os_label_impl();
        let keys = Self::key_chord_os_hint_impl();
        format!(
            "Desktop automation (host OS: {}). {} \
The **primary model cannot consume images** in tool results — **do not** use **`screenshot`**.\n\
**ACTION PRIORITY (CRITICAL):** Always think in this order:\n\
1. **Terminal/CLI/System commands first** — Use Bash tool for terminal commands, system scripts (e.g., macOS `osascript`), shell automation. Most efficient.\n\
2. **Keyboard shortcuts second** — Use **`key_chord`** / **`type_text`** for system/app shortcuts, navigation keys.\n\
3. **Precise UI control last** — Only when above fail: **`click_target`** / **`move_to_target`** (AX → OCR → screen coords in one call) → lower-level **`click_element`** / **`move_to_text`** → **`mouse_move`** + **`click`**.\n\
**Rhythm:** one action at a time; use **`wait`** when UI animates. Observe **`interaction_state`** and **`computer_use_context`** in tool JSON.\n\
**`click_target` / `move_to_target`:** Unified resolver: AX filters or `target_text` first, OCR second, explicit global x/y last. **`click_element` / `locate`:** Accessibility (AX/UIA/AT-SPI). **`move_to_text`:** OCR match + move pointer only. **`click`:** at current pointer only — use **`mouse_move`** or **`move_to_text`** / **`click_element`** first.\n\
**`mouse_move` / `drag`:** **`use_screen_coordinates`: true** with globals from tools. **`pointer_move_rel`:** relative nudge; host may block right after certain flows — follow tool errors.\n\
**`key_chord` / `type_text` / `scroll` / `wait`:** standard desktop automation without any screenshot step.\n",
            os, keys
        )
    }

    /// Tool description for the multimodal (vision-capable) model — full action surface.
    pub(crate) async fn description_impl() -> NortHingResult<String> {
        let os = Self::host_os_label_impl();
        let keys = Self::key_chord_os_hint_impl();
        Ok(format!(
            "Desktop automation (host OS: {}). {} All actions in one tool. Send only parameters that apply to the chosen `action`. \
**ACTION PRIORITY (CRITICAL):** Always think in this order before choosing an action:\n\
1. **Terminal/CLI/System commands first** — Use Bash tool for terminal commands, system scripts (e.g., macOS `osascript`, AppleScript), shell automation. This is the MOST EFFICIENT approach.\n\
2. **Keyboard shortcuts second** — Use **`key_chord`** for system shortcuts, app shortcuts, navigation keys (Enter, Escape, Tab, Space, Arrow keys). Prefer over mouse when equivalent.\n\
3. **Precise UI control last** — Only when above methods fail: prefer **`click_target`** / **`move_to_target`** (AX → OCR → screen coords in one call). Use lower-level **`click_element`**, **`move_to_text`**, or **`mouse_move`** + **`click`** only when you need manual disambiguation.\n\
**Screenshot usage:** **`screenshot`** is ONLY for observing/confirming UI state and extracting text/information — NEVER use screenshot coordinates to control mouse movement. Always use precise methods (AX, OCR, system coordinates) for targeting.\n\
**Cowork-style loop:** **`screenshot`** (observe) → **one** action → **`screenshot`** (verify). Use **`wait`** if UI animates. When **`interaction_state.recommend_screenshot_to_verify_last_action`** is true, call **`screenshot`** next. \
**`click_target` / `move_to_target`:** Unified target resolver. In one call it tries AX (`node_idx`, `text_contains`, `title_contains`, `role_substring`, `identifier_contains`, or `target_text`) first, then OCR (`target_text` / `text_query`), then explicit global `x`/`y` with `use_screen_coordinates: true`. `click_target` moves and clicks authoritatively, avoiding the multi-step locate → move → screenshot → click loop for common targets. \
**`click_element`:** Lower-level Accessibility tree (AX/UIA/AT-SPI) locate + click. Provide `title_contains` / `role_substring` / `identifier_contains`. On macOS, **`TextArea`** and **`TextField`** match both `AXTextArea` and `AXTextField` (many chat apps use TextField for compose). If several text fields match, the host deprioritizes known **search** controls (e.g. WeChat `_SC_SEARCH_FIELD`) and prefers **lower** on-screen fields (composer). Bypasses coordinate screenshot guard. \
**`move_to_text`:** OCR-match visible text (`text_query`) and **move the pointer** to it (no click, no keys); **no prior `screenshot` required for targeting** (host captures **raw** pixels for Vision — no agent screenshot overlays; on macOS defaults to the **frontmost window** unless **`ocr_region_native`** overrides). Matching **strips whitespace** between CJK glyphs and allows **small edit distance** when Vision mis-reads one character. The host **trusts** the resulting globals — **next `click`** does **not** require an extra `screenshot` (same as AX). If **several** hits match, the host returns **preview JPEGs + accessibility** per candidate — pick **`move_to_text_match_index`** (1-based) and call **`move_to_text`** again with the same query/region, or narrow with **`ocr_region_native`**. Use **`click`** afterward if you need a mouse press. Prefer after `click_element` misses when text is visible. \
**`click`:** Press at **current pointer only** — **never** pass `x`, `y`, `coordinate_mode`, or `use_screen_coordinates`. Position first with **`move_to_text`**, **`mouse_move`** (**globals only**), or **`click_element`**. After pointer moves, **`screenshot`** again before the next guarded **`click`** when the host requires it. \
**`mouse_move` / `drag`:** **`use_screen_coordinates`: true** required — global coordinates from **`move_to_text`**, **`locate`**, AX, or **`pointer_global`**; never JPEG pixel guesses. \
**`scroll` / `type_text` / `pointer_move_rel` / `wait` / `locate`:** No mandatory pre-screenshot by themselves. **`pointer_move_rel`** (and **ComputerUseMouseStep**) are **blocked immediately after `screenshot`** until **`move_to_text`**, **`mouse_move`** (globals), or **`click_element`** — do not nudge from the JPEG. \
**`key_chord`:** Press key combination; prefer over **`click`** when shortcuts or **Enter**/**Escape**/**Tab** suffice. **Mandatory fresh screenshot only** when chord includes Return/Enter. \
**`screenshot`:** JPEG for **confirmation** (optional pointer overlay). When the host requires a fresh capture before **`click`** or Enter **`key_chord`**, a bare `screenshot` is **~500×500** around the **mouse** or **caret** (also during quadrant drill). Use **`screenshot_reset_navigation`**: true to force **full-screen** for wide context. \
**`type_text`:** Type text; prefer clipboard for long content. Does **not** move the pointer — **Enter** **`key_chord`** may follow without a mandatory `screenshot` unless you moved the pointer since the last capture. If **`screenshot`** shows the correct chat is already open and the input may be focused, **try `type_text` first** before spending steps on `click_element` / `move_to_text`.",
            os, keys,
        ))
    }

    pub(crate) fn short_description_impl() -> String {
        "Inspect the screen and control desktop input for computer-use tasks.".to_string()
    }

    /// JSON Schema without `screenshot` or screenshot-only fields.
    pub(crate) fn input_schema_text_only_impl() -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["click_target", "move_to_target", "click_element", "move_to_text", "click", "mouse_move", "scroll", "drag", "locate", "key_chord", "type_text", "pointer_move_rel", "wait", "list_displays", "focus_display", "paste", "list_apps", "get_app_state", "app_click", "app_type_text", "app_scroll", "app_key_chord", "app_wait_for", "build_interactive_view", "interactive_click", "interactive_type_text", "interactive_scroll", "build_visual_mark_view", "visual_click", "open_app", "open_url", "open_file", "clipboard_get", "clipboard_set", "run_script", "run_apple_script", "get_os_info"],
                    "description": "The action to perform. **Primary model is text-only — no `screenshot`.** **ACTION PRIORITY:** 1) Use Bash tool for CLI/terminal/system commands first. 2) **`open_app`** to launch apps. **`run_apple_script`** for AppleScript (macOS). 3) Prefer `key_chord` for shortcuts/navigation. 4) Only when above fail: `click_target` / `move_to_target` (AX → OCR → screen coords in one call), then lower-level `click_element`, `move_to_text`, or `mouse_move` + `click`. Never guess coordinates."
                },
                "x": { "type": "integer", "description": "For `mouse_move` and `drag`: X in **global display** units when **`use_screen_coordinates`: true** (required). **Not** for `click`." },
                "y": { "type": "integer", "description": "For `mouse_move` and `drag`: Y in **global display** units when **`use_screen_coordinates`: true** (required). **Not** for `click`." },
                "coordinate_mode": { "type": "string", "enum": ["image", "normalized"], "description": "Ignored for `mouse_move` / `drag` — host rejects image/normalized positioning; always set **`use_screen_coordinates`: true**." },
                "use_screen_coordinates": { "type": "boolean", "description": "For `mouse_move`, `drag`: **must be true** — global display coordinates from `move_to_text`, `locate`, AX, or `pointer_global`. **Not** for `click`." },
                "button": { "type": "string", "enum": ["left", "right", "middle"], "description": "For `click`, `click_element`, `drag`: mouse button (default left)." },
                "num_clicks": { "type": "integer", "minimum": 1, "maximum": 3, "description": "For `click`, `click_element`: 1=single (default), 2=double, 3=triple click." },
                "delta_x": { "type": "integer", "description": "For `pointer_move_rel`: horizontal delta (negative=left); also accepted as `dx`. For `scroll`: horizontal wheel delta." },
                "delta_y": { "type": "integer", "description": "For `pointer_move_rel`: vertical delta (negative=up); also accepted as `dy`. For `scroll`: vertical wheel delta." },
                "start_x": { "type": "integer", "description": "For `drag`: start X coordinate." },
                "start_y": { "type": "integer", "description": "For `drag`: start Y coordinate." },
                "end_x": { "type": "integer", "description": "For `drag`: end X coordinate." },
                "end_y": { "type": "integer", "description": "For `drag`: end Y coordinate." },
                "keys": { "type": "array", "items": { "type": "string" }, "description": "For `key_chord`: keys in order — modifiers first, then the main key. Desktop host waits after pressing modifiers so shortcuts register (important on macOS with IME)." },
                "text": { "type": "string", "description": "For `type_text`: text to type. Prefer clipboard paste (key_chord) for long content." },
                "ms": { "type": "integer", "description": "For `wait`: duration in milliseconds." },
                "target_text": { "type": "string", "description": "For `move_to_target` / `click_target`: visible or accessible text. The resolver tries AX first, then OCR." },
                "target_match_index": { "type": "integer", "minimum": 1, "description": "For `move_to_target` / `click_target`: optional 1-based OCR match index when you want a specific candidate." },
                "text_query": { "type": "string", "description": "For `move_to_text`, `move_to_target`, `click_target`: visible text to OCR-match on screen (case-insensitive substring)." },
                "move_to_text_match_index": { "type": "integer", "minimum": 1, "description": "For `move_to_text` and unified target actions: **1-based** OCR match index." },
                "ocr_region_native": {
                    "type": "object",
                    "description": "For `move_to_text`: optional global native rectangle for OCR. If omitted, macOS uses the frontmost window bounds from Accessibility; other OSes use the primary display.",
                    "properties": {
                        "x0": { "type": "integer", "description": "Top-left X in global screen coordinates." },
                        "y0": { "type": "integer", "description": "Top-left Y in global screen coordinates." },
                        "width": { "type": "integer", "minimum": 1, "description": "Width in the same coordinate unit as x0/y0." },
                        "height": { "type": "integer", "minimum": 1, "description": "Height in the same coordinate unit as x0/y0." }
                    }
                },
                "title_contains": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring on AXTitle ONLY. Prefer `text_contains` (also covers AXValue/AXDescription/AXHelp)." },
                "role_substring": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring on AXRole **or AXSubrole** (e.g. \"Button\", \"SearchField\")." },
                "identifier_contains": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring on AXIdentifier." },
                "text_contains": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring matched against ANY of AXTitle / AXValue / AXDescription / AXHelp. Prefer this when the visible text is shown via value/description (e.g. AXStaticText cards) instead of title." },
                "node_idx": { "type": "integer", "minimum": 0, "description": "For `locate`, `click_element`, `app_click`: jump straight to a node returned by the most recent `get_app_state` (field `idx`). Bypasses BFS. macOS only; other platforms return AX_IDX_NOT_SUPPORTED." },
                "app_state_digest": { "type": "string", "description": "For `locate`, `click_element`: optional `state_digest` from the same `get_app_state` call that produced `node_idx`. Stale digest yields AX_IDX_STALE so you re-snapshot." },
                "max_depth": { "type": "integer", "minimum": 1, "maximum": 200, "description": "For `locate`, `click_element`: max BFS depth (default 48). Ignored when `node_idx` is supplied." },
                "filter_combine": { "type": "string", "enum": ["all", "any"], "description": "For `locate`, `click_element`: `all` (default, AND) or `any` (OR) for filter combination. Priority: `node_idx` > `text_contains` > `title_contains`+`role_substring`." },
                "app_name": { "type": "string", "description": "For `open_app`: the application name to launch." },
                "url": { "type": "string", "description": "For `open_url`: URL to open with the system/default browser." },
                "path": { "type": "string", "description": "For `open_file`: local file path to open with its default handler." },
                "app": { "type": ["string", "object"], "description": "For `open_file`: optional app name. For app-scoped actions: selector object such as `{ \"name\": \"Safari\" }`, `{ \"bundle_id\": \"...\" }`, or `{ \"pid\": 123 }`." },
                "script": { "type": "string", "description": "For `run_apple_script`: the AppleScript code to execute. macOS only." },
                "script_type": { "type": "string", "enum": ["applescript", "shell", "bash", "powershell", "cmd"], "description": "For `run_script`: script interpreter/type." },
                "timeout_ms": { "type": "integer", "description": "For `run_script`: timeout in milliseconds." },
                "max_output_bytes": { "type": "integer", "description": "For `run_script` / `clipboard_get`: maximum bytes to return." },
                "clear_first": { "type": "boolean", "description": "For `paste`: select all before pasting." },
                "submit": { "type": "boolean", "description": "For `paste`: press submit keys after pasting." },
                "submit_keys": { "type": "array", "items": { "type": "string" }, "description": "For `paste`: key chord to submit, default `[\"return\"]`." },
                "display_id": { "type": ["integer", "null"], "description": "For `focus_display` or display-pinned desktop actions: display id, or null to clear the pin." },
                "include_hidden": { "type": "boolean", "description": "For `list_apps`: include hidden/background apps." },
                "only_visible": { "type": "boolean", "description": "For `list_apps`: list only visible apps when true." },
                "target": { "type": "object", "description": "For `app_click`: click target such as `{ \"node_idx\": 3 }`, image/screen coordinates, or OCR text." },
                "focus": { "type": ["object", "null"], "description": "For app-scoped text/scroll actions: optional focus target." },
                "predicate": { "type": "object", "description": "For `app_wait_for`: wait predicate." },
                "opts": { "type": "object", "description": "For `build_interactive_view` / `build_visual_mark_view`: optional view options." },
                "i": { "type": ["integer", "null"], "description": "For interactive/visual actions: element or mark index from the latest view." },
                "dx": { "type": "integer", "description": "For app/interactive scroll actions: horizontal delta." },
                "dy": { "type": "integer", "description": "For app/interactive scroll actions: vertical delta." },
                "mouse_button": { "type": "string", "enum": ["left", "right", "middle"], "description": "For app/interactive/visual click actions." },
                "click_count": { "type": "integer", "minimum": 1, "maximum": 3, "description": "For app click actions." },
                "modifier_keys": { "type": "array", "items": { "type": "string" }, "description": "For app click actions: modifier keys to hold." },
                "wait_ms_after": { "type": "integer", "description": "For app click actions: post-click wait in milliseconds." },
                "focus_idx": { "type": "integer", "minimum": 0, "description": "For `app_key_chord`: optional node index to focus first." },
                "poll_ms": { "type": "integer", "description": "For `app_wait_for`: polling interval." },
                "scroll_x": { "type": "integer", "description": "For `scroll`: optional global X coordinate to scroll at. Use with `scroll_y`." },
                "scroll_y": { "type": "integer", "description": "For `scroll`: optional global Y coordinate to scroll at. Use with `scroll_x`." }
            },
            "required": ["action"],
            "additionalProperties": false
        })
    }

    /// Full JSON schema (with `screenshot` + screenshot-only fields).
    pub(crate) fn input_schema_impl() -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["screenshot", "click_target", "move_to_target", "click_element", "move_to_text", "click", "mouse_move", "scroll", "drag", "locate", "key_chord", "type_text", "pointer_move_rel", "wait", "list_displays", "focus_display", "paste", "list_apps", "get_app_state", "app_click", "app_type_text", "app_scroll", "app_key_chord", "app_wait_for", "build_interactive_view", "interactive_click", "interactive_type_text", "interactive_scroll", "build_visual_mark_view", "visual_click", "open_app", "open_url", "open_file", "clipboard_get", "clipboard_set", "run_script", "run_apple_script", "get_os_info"],
                    "description": "The action to perform. **ACTION PRIORITY:** 1) Use Bash tool for CLI/terminal/system commands (most efficient). 2) **`open_app`** to launch apps by name. **`run_apple_script`** to run AppleScript (macOS). 3) Prefer **`key_chord`** for shortcuts/navigation keys over mouse. 4) Only when above fail: `click_target` / `move_to_target` (AX → OCR → screen coords in one call) before lower-level `click_element`, `move_to_text`, or `mouse_move` + `click`. **`screenshot`** is for observation/confirmation ONLY — never derive mouse coordinates from screenshots. `click` = press at **current pointer only** (no x/y params). `scroll` supports optional position (`scroll_x`/`scroll_y`). `type_text`, `drag`, `pointer_move_rel`, `wait`, `locate` = standard actions."
                },
                "x": { "type": "integer", "description": "For `mouse_move` and `drag`: X in **global display** units when **`use_screen_coordinates`: true** (required). **Not** for `click`." },
                "y": { "type": "integer", "description": "For `mouse_move` and `drag`: Y in **global display** units when **`use_screen_coordinates`: true** (required). **Not** for `click`." },
                "coordinate_mode": { "type": "string", "enum": ["image", "normalized"], "description": "Ignored for `mouse_move` / `drag` — host rejects image/normalized positioning; always set **`use_screen_coordinates`: true**." },
                "use_screen_coordinates": { "type": "boolean", "description": "For `mouse_move`, `drag`: **must be true** — global display coordinates (e.g. macOS points) from `move_to_text`, `locate`, AX, or `pointer_global`. **Not** for `click`." },
                "button": { "type": "string", "enum": ["left", "right", "middle"], "description": "For `click`, `click_element`, `drag`: mouse button (default left)." },
                "num_clicks": { "type": "integer", "minimum": 1, "maximum": 3, "description": "For `click`, `click_element`: 1=single (default), 2=double, 3=triple click." },
                "delta_x": { "type": "integer", "description": "For `pointer_move_rel`: horizontal delta (negative=left); also accepted as `dx`. **Not** allowed as the first move after `screenshot` (host). For `scroll`: horizontal wheel delta." },
                "delta_y": { "type": "integer", "description": "For `pointer_move_rel`: vertical delta (negative=up); also accepted as `dy`. **Not** allowed as the first move after `screenshot` (host). For `scroll`: vertical wheel delta." },
                "start_x": { "type": "integer", "description": "For `drag`: start X coordinate." },
                "start_y": { "type": "integer", "description": "For `drag`: start Y coordinate." },
                "end_x": { "type": "integer", "description": "For `drag`: end X coordinate." },
                "end_y": { "type": "integer", "description": "For `drag`: end Y coordinate." },
                "keys": { "type": "array", "items": { "type": "string" }, "description": "For `key_chord`: keys in order — **modifiers first**, then the main key (e.g. `[\"command\",\"f\"]`). Desktop host waits after pressing modifiers so shortcuts register (important on macOS with IME). Modifiers: command, control, shift, alt/option. Arrows: `up`, `down`, … Host may require a fresh screenshot before Return/Enter when the pointer is stale." },
                "text": { "type": "string", "description": "For `type_text`: text to type. Prefer clipboard paste (key_chord) for long content." },
                "ms": { "type": "integer", "description": "For `wait`: duration in milliseconds." },
                "target_text": { "type": "string", "description": "For `move_to_target` / `click_target`: visible or accessible text. The resolver tries AX text first, then OCR text, without requiring a prior screenshot." },
                "target_match_index": { "type": "integer", "minimum": 1, "description": "For `move_to_target` / `click_target`: optional 1-based OCR match index when you want a specific candidate. Alias of `move_to_text_match_index` for the unified target actions." },
                "text_query": { "type": "string", "description": "For `move_to_text`, `move_to_target`, `click_target`: visible text to OCR-match on screen (case-insensitive substring)." },
                "move_to_text_match_index": { "type": "integer", "minimum": 1, "description": "For `move_to_text` and unified target actions: **1-based** OCR match index. For `move_to_text`, use after a disambiguation response; for `click_target`, use to pin a candidate." },
                "ocr_region_native": {
                    "type": "object",
                    "description": "For `move_to_text`: optional global native rectangle for OCR. If omitted, macOS uses the frontmost window bounds from Accessibility; other OSes use the primary display. Overrides the automatic region when set. Requires x0, y0, width, height.",
                    "properties": {
                        "x0": { "type": "integer", "description": "Top-left X in global screen coordinates (macOS: same logical space as CGDisplayBounds / pointer; not physical Retina pixels)." },
                        "y0": { "type": "integer", "description": "Top-left Y in global screen coordinates (macOS: logical, Y-down)." },
                        "width": { "type": "integer", "minimum": 1, "description": "Width in the same coordinate unit as x0/y0 (logical on macOS)." },
                        "height": { "type": "integer", "minimum": 1, "description": "Height in the same coordinate unit as x0/y0 (logical on macOS)." }
                    }
                },
                "title_contains": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring on AXTitle ONLY. Use same language as the app UI. Prefer `text_contains` (also covers AXValue/AXDescription/AXHelp) when in doubt." },
                "role_substring": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring on AXRole **or AXSubrole** (e.g. \"Button\", \"TextField\", \"SearchField\")." },
                "identifier_contains": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring on AXIdentifier." },
                "text_contains": { "type": "string", "description": "For `locate`, `click_element`: case-insensitive substring matched against ANY of AXTitle / AXValue / AXDescription / AXHelp. Best default when the visible label lives in value/description (e.g. AXStaticText cards)." },
                "node_idx": { "type": "integer", "minimum": 0, "description": "For `locate`, `click_element`, `app_click`: jump straight to a node returned by the most recent `get_app_state` (field `idx`). Bypasses BFS. macOS only; other platforms return AX_IDX_NOT_SUPPORTED." },
                "app_state_digest": { "type": "string", "description": "For `locate`, `click_element`: optional `state_digest` from the same `get_app_state` call that produced `node_idx`. Stale digest yields AX_IDX_STALE so you re-snapshot." },
                "max_depth": { "type": "integer", "minimum": 1, "maximum": 200, "description": "For `locate`, `click_element`: max BFS depth (default 48). Ignored when `node_idx` is supplied." },
                "filter_combine": { "type": "string", "enum": ["all", "any"], "description": "For `locate`, `click_element`: `all` (default, AND) or `any` (OR) for filter combination. Priority: `node_idx` > `text_contains` > `title_contains`+`role_substring`." },
                "screenshot_crop_center_x": { "type": "integer", "minimum": 0, "description": "For `screenshot`: point crop X center in full-capture native pixels." },
                "screenshot_crop_center_y": { "type": "integer", "minimum": 0, "description": "For `screenshot`: point crop Y center in full-capture native pixels." },
                "screenshot_crop_half_extent_native": { "type": "integer", "minimum": 0, "description": "For `screenshot`: half-size of point crop in native pixels (default 250)." },
                "screenshot_navigate_quadrant": { "type": "string", "enum": ["top_left", "top_right", "bottom_left", "bottom_right"], "description": "For `screenshot`: zoom into quadrant. Repeat until `quadrant_navigation_click_ready` is true." },
                "screenshot_reset_navigation": { "type": "boolean", "description": "For `screenshot`: reset to full display before this capture." },
                "screenshot_implicit_center": { "type": "string", "enum": ["mouse", "text_caret"], "description": "For `screenshot` when `requires_fresh_screenshot_before_click` / `requires_fresh_screenshot_before_enter` is true: center the implicit ~500×500 on the mouse (`mouse`, default) or on the focused text control (`text_caret`, macOS AX; falls back to mouse). Applies to the **first** confirmation capture too. Ignored when you set `screenshot_crop_center_*` / `screenshot_navigate_quadrant` / `screenshot_reset_navigation`." },
                "app_name": { "type": "string", "description": "For `open_app`: the application name to launch (e.g. \"Safari\", \"WeChat\", \"Visual Studio Code\")." },
                "url": { "type": "string", "description": "For `open_url`: URL to open with the system/default browser." },
                "path": { "type": "string", "description": "For `open_file`: local file path to open with its default handler." },
                "app": { "type": ["string", "object"], "description": "For `open_file`: optional app name. For app-scoped actions: selector object such as `{ \"name\": \"Safari\" }`, `{ \"bundle_id\": \"...\" }`, or `{ \"pid\": 123 }`." },
                "script": { "type": "string", "description": "For `run_apple_script`: the AppleScript code to execute via `osascript`. macOS only." },
                "script_type": { "type": "string", "enum": ["applescript", "shell", "bash", "powershell", "cmd"], "description": "For `run_script`: script interpreter/type." },
                "timeout_ms": { "type": "integer", "description": "For `run_script`: timeout in milliseconds." },
                "max_output_bytes": { "type": "integer", "description": "For `run_script` / `clipboard_get`: maximum bytes to return." },
                "clear_first": { "type": "boolean", "description": "For `paste`: select all before pasting." },
                "submit": { "type": "boolean", "description": "For `paste`: press submit keys after pasting." },
                "submit_keys": { "type": "array", "items": { "type": "string" }, "description": "For `paste`: key chord to submit, default `[\"return\"]`." },
                "display_id": { "type": ["integer", "null"], "description": "For `focus_display` or display-pinned desktop actions: display id, or null to clear the pin." },
                "include_hidden": { "type": "boolean", "description": "For `list_apps`: include hidden/background apps." },
                "only_visible": { "type": "boolean", "description": "For `list_apps`: list only visible apps when true." },
                "target": { "type": "object", "description": "For `app_click`: click target such as `{ \"node_idx\": 3 }`, image/screen coordinates, or OCR text." },
                "focus": { "type": ["object", "null"], "description": "For app-scoped text/scroll actions: optional focus target." },
                "predicate": { "type": "object", "description": "For `app_wait_for`: wait predicate." },
                "opts": { "type": "object", "description": "For `build_interactive_view` / `build_visual_mark_view`: optional view options." },
                "i": { "type": ["integer", "null"], "description": "For interactive/visual actions: element or mark index from the latest view." },
                "dx": { "type": "integer", "description": "For app/interactive scroll actions: horizontal delta." },
                "dy": { "type": "integer", "description": "For app/interactive scroll actions: vertical delta." },
                "mouse_button": { "type": "string", "enum": ["left", "right", "middle"], "description": "For app/interactive/visual click actions." },
                "click_count": { "type": "integer", "minimum": 1, "maximum": 3, "description": "For app click actions." },
                "modifier_keys": { "type": "array", "items": { "type": "string" }, "description": "For app click actions: modifier keys to hold." },
                "wait_ms_after": { "type": "integer", "description": "For app click actions: post-click wait in milliseconds." },
                "focus_idx": { "type": "integer", "minimum": 0, "description": "For `app_key_chord`: optional node index to focus first." },
                "poll_ms": { "type": "integer", "description": "For `app_wait_for`: polling interval." },
                "scroll_x": { "type": "integer", "description": "For `scroll`: optional global X coordinate to move pointer before scrolling. Use with `scroll_y`. Requires `use_screen_coordinates`: true." },
                "scroll_y": { "type": "integer", "description": "For `scroll`: optional global Y coordinate to move pointer before scrolling. Use with `scroll_x`. Requires `use_screen_coordinates`: true." }
            },
            "required": ["action"],
            "additionalProperties": false
        })
    }

    /// True when the action has been migrated to the ControlHub desktop
    /// domain (`ComputerUseActions::handle_desktop`).
    pub(crate) fn is_controlhub_migrated_desktop_action(action: &str) -> bool {
        matches!(
            action,
            "list_displays"
                | "focus_display"
                | "paste"
                | "list_apps"
                | "get_app_state"
                | "app_click"
                | "app_type_text"
                | "app_scroll"
                | "app_key_chord"
                | "app_wait_for"
                | "build_interactive_view"
                | "interactive_click"
                | "interactive_type_text"
                | "interactive_scroll"
                | "build_visual_mark_view"
                | "visual_click"
        )
    }

    /// Lower-case provider name (used to gate multimodal tool output).
    pub(crate) fn primary_api_format_impl(ctx: &ToolUseContext) -> String {
        ctx.custom_data
            .get("primary_model_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase()
    }

    /// Screenshot tool results attach JPEGs via `tool_image_attachments`; only providers whose
    /// request converters emit multimodal tool output are supported (Anthropic + OpenAI-compatible).
    pub(crate) fn require_multimodal_tool_output_for_screenshot_impl(ctx: &ToolUseContext) -> NortHingResult<()> {
        if !ctx.primary_model_supports_image_understanding() {
            return Err(NortHingError::tool(
                "The primary model does not accept images; do not use ComputerUse action `screenshot` or other image-producing steps. Use `click_element`, `locate`, `move_to_text` (with `move_to_text_match_index` when listed), `mouse_move` with globals from tool JSON, `key_chord`, etc.".to_string(),
            ));
        }
        let f = Self::primary_api_format_impl(ctx);
        if matches!(f.as_str(), "anthropic" | "openai" | "response" | "responses") {
            return Ok(());
        }
        Err(NortHingError::tool(
            "Screenshot results include images in tool results; set the primary model to Anthropic (Claude) or OpenAI-compatible API format. Other providers are not supported for screenshots yet.".to_string(),
        ))
    }

    /// Runtime host OS label for tool description (desktop session matches this process).
    pub(crate) fn host_os_label_impl() -> &'static str {
        match std::env::consts::OS {
            "macos" => "macOS",
            "windows" => "Windows",
            "linux" => "Linux",
            other => other,
        }
    }

    pub(crate) fn key_chord_os_hint_impl() -> &'static str {
        match std::env::consts::OS {
            "macos" => "On this host use command/option/control/shift in key_chord (not Win/Linux names). **System clipboard (prefer over type_text when pasting):** command+a select all, command+c copy, command+x cut, command+v paste — combine with focus/selection shortcuts as needed.",
            "windows" => "On this host use meta (Windows key), alt, control, shift in key_chord. **System clipboard:** control+a/c/x/v for select all, copy, cut, paste.",
            "linux" => "On this host use control, alt, shift, and meta/super as appropriate for the desktop. **System clipboard:** typically control+a/c/x/v (match the app and DE).",
            _ => "Match key_chord modifiers to the host OS in Runtime Context. Prefer standard clipboard chords (select all, copy, cut, paste) before long type_text.",
        }
    }
}
