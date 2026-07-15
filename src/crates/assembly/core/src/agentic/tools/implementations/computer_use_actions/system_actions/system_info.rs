//! OS info and platform discovery handler.

use crate::agentic::tools::framework::ToolResult;
#[cfg(target_os = "linux")]
use crate::agentic::tools::implementations::computer_use_actions::utilities::linux_session_info;
use crate::agentic::tools::implementations::computer_use_actions::utilities::{
    hostname, read_os_version, which_exists,
};
use crate::agentic::tools::implementations::computer_use_actions::ComputerUseActions;
use crate::agentic::tools::implementations::control_hub::{err_response, ControlHubError, ErrorCode};
use crate::util::errors::NortHingResult;
use serde_json::json;

impl ComputerUseActions {
    pub(crate) async fn handle_get_os_info(
        &self,
        _params: &serde_json::Value,
        _context: &crate::agentic::tools::framework::ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let mut info = json!({
            "os": os,
            "arch": arch,
            "rust_target_family": std::env::consts::FAMILY,
        });
        if let Some(v) = read_os_version() {
            info["os_version"] = json!(v);
        }
        if let Ok(host) = hostname() {
            info["hostname"] = json!(host);
        }
        #[cfg(target_os = "linux")]
        {
            let (display_server, desktop_env) = linux_session_info();
            if let Some(s) = display_server {
                info["display_server"] = json!(s);
            }
            if let Some(d) = desktop_env {
                info["desktop_environment"] = json!(d);
            }
        }
        let mut script_types = vec!["shell"];
        if cfg!(target_os = "macos") {
            script_types.push("applescript");
        }
        if which_exists("bash") {
            script_types.push("bash");
        }
        if which_exists("pwsh") || which_exists("powershell") {
            script_types.push("powershell");
        }
        if cfg!(target_os = "windows") {
            script_types.push("cmd");
        }
        info["script_types"] = json!(script_types);
        Ok(vec![ToolResult::ok(
            info.clone(),
            Some(format!(
                "{} {} ({})",
                os,
                info.get("os_version").and_then(|v| v.as_str()).unwrap_or(""),
                arch
            )),
        )])
    }
}
