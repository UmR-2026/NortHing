use super::super::builtin_clients::builtin_acp_client_preset;
use super::super::config::AcpClientConfig;

pub(crate) struct AcpRequirementSpec<'a> {
    pub(crate) tool_command: &'a str,
    pub(crate) install_package: Option<&'a str>,
    pub(crate) adapter: Option<AcpAdapterSpec<'a>>,
}

pub(crate) struct AcpAdapterSpec<'a> {
    pub(crate) package: &'a str,
    pub(crate) bin: &'a str,
}

pub(crate) fn acp_requirement_spec<'a>(
    client_id: &'a str,
    config: Option<&'a AcpClientConfig>,
) -> AcpRequirementSpec<'a> {
    if let Some(preset) = builtin_acp_client_preset(client_id) {
        return AcpRequirementSpec {
            tool_command: preset.tool_command,
            install_package: preset.install_package,
            adapter: match (preset.adapter_package, preset.adapter_bin) {
                (Some(package), Some(bin)) => Some(AcpAdapterSpec { package, bin }),
                _ => None,
            },
        };
    }

    AcpRequirementSpec {
        tool_command: config.map(|config| config.command.as_str()).unwrap_or(client_id),
        install_package: None,
        adapter: None,
    }
}
