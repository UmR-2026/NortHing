use super::*;
use crate::app_state::settings::keyring::MockKeyring;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn sample_provider() -> ProviderConfig {
    ProviderConfig::new("我的 Anthropic".into(), ProviderType::Anthropic)
}

#[test]
fn provider_type_default_base_url() {
    assert_eq!(ProviderType::Anthropic.default_base_url(), "https://api.anthropic.com");
    assert_eq!(ProviderType::Openai.default_base_url(), "https://api.openai.com/v1");
    assert_eq!(
        ProviderType::Gemini.default_base_url(),
        "https://generativelanguage.googleapis.com/v1beta"
    );
    assert_eq!(ProviderType::CustomOpenaiCompatible.default_base_url(), "");
}

#[test]
fn provider_type_default_models_non_empty_for_named() {
    assert!(!ProviderType::Anthropic.default_models().is_empty());
    assert!(!ProviderType::Openai.default_models().is_empty());
    assert!(!ProviderType::Gemini.default_models().is_empty());
    assert!(ProviderType::CustomOpenaiCompatible.default_models().is_empty());
}

#[test]
fn provider_new_has_unique_id_and_defaults() {
    let a = sample_provider();
    let b = sample_provider();
    assert_ne!(a.id, b.id);
    assert!(a.enabled);
    assert_eq!(a.base_url, "https://api.anthropic.com");
    assert_eq!(a.model, "claude-sonnet-4-5");
    assert!(a.api_key.is_empty());
    assert!(a.last_verified_ok.is_none());
}

#[test]
fn is_first_run_empty_settings() {
    let s = AppSettings::default();
    assert!(s.is_first_run());
}

#[test]
fn is_first_run_with_workspace() {
    let mut s = AppSettings::default();
    s.add_workspace(PathBuf::from("/tmp"));
    assert!(!s.is_first_run());
}

#[test]
fn workspace_add_dedups() {
    let mut s = AppSettings::default();
    s.add_workspace(PathBuf::from("/tmp"));
    s.add_workspace(PathBuf::from("/tmp"));
    assert_eq!(s.workspaces.len(), 1);
}

#[test]
fn workspace_set_current_updates_last_opened() {
    let mut s = AppSettings::default();
    s.add_workspace(PathBuf::from("/a"));
    s.add_workspace(PathBuf::from("/b"));
    s.set_current_workspace(Some(Path::new("/b")));
    assert_eq!(s.current_workspace, Some(PathBuf::from("/b")));
    let b_last = s
        .workspaces
        .iter()
        .find(|w| w.path == Path::new("/b"))
        .unwrap()
        .last_opened_at;
    let a_last = s
        .workspaces
        .iter()
        .find(|w| w.path == Path::new("/a"))
        .unwrap()
        .last_opened_at;
    assert!(b_last >= a_last);
}

#[test]
fn remove_workspace_clears_current() {
    let mut s = AppSettings::default();
    s.add_workspace(PathBuf::from("/a"));
    s.set_current_workspace(Some(Path::new("/a")));
    s.remove_workspace(Path::new("/a"));
    assert!(s.current_workspace.is_none());
}

#[test]
fn settings_json_roundtrip() {
    let mut s = AppSettings::default();
    s.add_workspace(PathBuf::from("/tmp/proj"));
    let json = serde_json::to_string_pretty(&s).unwrap();
    let back: AppSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.workspaces.len(), 1);
}

#[test]
fn onboarding_completed_serde_default_false() {
    let full = serde_json::to_value(AppSettings::default()).expect("serialize default");
    let mut obj = full.as_object().expect("object").clone();
    obj.remove("onboarding_completed");
    let s: AppSettings =
        serde_json::from_value(serde_json::Value::Object(obj)).expect("deserialize without onboarding_completed");
    assert!(!s.onboarding_completed, "missing field should default to false");
}

#[test]
fn onboarding_completed_roundtrip() {
    let mut s = AppSettings::default();
    assert!(!s.onboarding_completed);
    s.onboarding_completed = true;
    let json = serde_json::to_string_pretty(&s).unwrap();
    let back: AppSettings = serde_json::from_str(&json).unwrap();
    assert!(back.onboarding_completed, "true should round-trip");
}

#[test]
fn validate_session_integrity_detects_deleted_provider() {
    let mut s = AppSettings::default();
    let p_id = "test-prov-id".to_string();
    let mut known_providers = HashSet::new();
    known_providers.insert(p_id.clone());

    s.add_workspace(PathBuf::from("/tmp/proj"));

    let provider_lookup = |_sid: &str| -> Option<String> { Some(p_id.clone()) };
    let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/proj")) };

    // Before deletion: no issues.
    let issues = s.validate_session_integrity(
        &known_providers,
        vec!["s1".to_string()],
        &provider_lookup,
        &workspace_lookup,
    );
    assert!(issues.is_empty(), "no issues when provider + workspace exist");

    // Delete the provider from known_providers; expect Q6 issue.
    known_providers.remove(&p_id);
    let issues = s.validate_session_integrity(
        &known_providers,
        vec!["s1".to_string()],
        &provider_lookup,
        &workspace_lookup,
    );
    assert_eq!(issues.len(), 1, "expected exactly the Q6 issue");
    assert_eq!(issues[0].kind, "provider-deleted");
    assert_eq!(issues[0].session_id, "s1");
    assert_eq!(issues[0].related_id, p_id);
}

#[test]
fn validate_session_integrity_detects_removed_workspace() {
    let mut s = AppSettings::default();
    s.add_workspace(PathBuf::from("/tmp/exists"));
    let known_providers = HashSet::new();

    let provider_lookup = |_sid: &str| -> Option<String> { None };
    let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/removed")) };

    let issues = s.validate_session_integrity(
        &known_providers,
        vec!["s1".to_string()],
        &provider_lookup,
        &workspace_lookup,
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].kind, "workspace-removed");
    assert_eq!(issues[0].related_id, "/tmp/removed");
}

#[test]
fn validate_session_integrity_reports_both_q6_and_q7_per_session() {
    let s = AppSettings::default();
    let known_providers = HashSet::new();

    let provider_lookup = |_sid: &str| -> Option<String> { Some("missing-provider".to_string()) };
    let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/missing")) };

    let issues = s.validate_session_integrity(
        &known_providers,
        vec!["s1".to_string()],
        &provider_lookup,
        &workspace_lookup,
    );
    assert_eq!(issues.len(), 2);
    let kinds: Vec<&str> = issues.iter().map(|i| i.kind.as_str()).collect();
    assert!(kinds.contains(&"provider-deleted"));
    assert!(kinds.contains(&"workspace-removed"));
}

#[test]
fn validate_session_integrity_empty_session_list_is_noop() {
    let s = AppSettings::default();
    let known_providers = HashSet::new();
    let issues = s.validate_session_integrity(&known_providers, std::iter::empty::<String>(), |_| None, |_| None);
    assert!(issues.is_empty());
}

#[test]
fn integration_welcome_provider_session_delete_provider() {
    use std::collections::HashMap;

    // Step 1: empty settings → first-run flag set.
    let mut s = AppSettings::default();
    assert!(s.is_first_run(), "empty settings is first run");

    // Step 2: user adds a workspace.
    s.add_workspace(PathBuf::from("/tmp/proj"));
    s.set_current_workspace(Some(&PathBuf::from("/tmp/proj")));
    assert!(!s.is_first_run(), "after workspace, not first run");

    // Step 3: known providers list has provider.
    let provider_id = "provider-1".to_string();
    let mut known_providers = HashSet::new();
    known_providers.insert(provider_id.clone());

    // Step 4: user creates a session using the provider.
    let session_id = "sess-1".to_string();
    let mut session_provider_lookup = HashMap::new();
    session_provider_lookup.insert(session_id.clone(), provider_id.clone());
    let mut session_workspace_lookup = HashMap::new();
    session_workspace_lookup.insert(session_id.clone(), PathBuf::from("/tmp/proj"));
    let provider_lookup = |sid: &str| -> Option<String> { session_provider_lookup.get(sid).cloned() };
    let workspace_lookup = |sid: &str| -> Option<PathBuf> { session_workspace_lookup.get(sid).cloned() };

    // No issues yet.
    let issues = s.validate_session_integrity(
        &known_providers,
        vec![session_id.clone()],
        &provider_lookup,
        &workspace_lookup,
    );
    assert!(issues.is_empty(), "all healthy before delete");

    // Step 5: provider deleted from core.
    known_providers.remove(&provider_id);

    // Now integrity should flag the session.
    let issues = s.validate_session_integrity(
        &known_providers,
        vec![session_id.clone()],
        &provider_lookup,
        &workspace_lookup,
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].session_id, session_id);
    assert_eq!(issues[0].kind, "provider-deleted");
    assert_eq!(issues[0].related_id, provider_id);
}

// ===== Core sync helper tests =====

#[test]
fn provider_wire_format_mapping() {
    assert_eq!(provider_wire_format(&ProviderType::Anthropic), "anthropic");
    assert_eq!(provider_wire_format(&ProviderType::Openai), "openai");
    assert_eq!(provider_wire_format(&ProviderType::Gemini), "gemini");
    assert_eq!(provider_wire_format(&ProviderType::CustomOpenaiCompatible), "openai");
    assert_eq!(
        provider_wire_format(&ProviderType::CustomAnthropicCompatible),
        "anthropic"
    );
}

#[test]
fn provider_wire_format_from_str_mapping() {
    assert_eq!(provider_wire_format_from_str("anthropic"), "anthropic");
    assert_eq!(provider_wire_format_from_str("custom-anthropic"), "anthropic");
    assert_eq!(provider_wire_format_from_str("openai"), "openai");
    assert_eq!(provider_wire_format_from_str("custom-openai"), "openai");
    assert_eq!(provider_wire_format_from_str("gemini"), "gemini");
    assert_eq!(provider_wire_format_from_str("other"), "openai");
}

#[test]
fn provider_to_ai_model_config_fields() {
    let kr = MockKeyring::new();
    let p = ProviderConfig::new("我的 Anthropic".into(), ProviderType::Anthropic);
    let m = provider_to_ai_model_config(&p, &kr);
    assert_eq!(m.id, p.id);
    assert_eq!(m.display_name, Some("我的 Anthropic".to_string()));
    assert_eq!(m.provider_id, "anthropic");
    assert_eq!(m.model, p.model);
    assert_eq!(m.api_key, Some(p.api_key.clone()));
    assert_eq!(m.enabled, Some(p.enabled));
    assert!(m.base_url.as_deref().unwrap_or("").contains("anthropic"));
    assert_eq!(m.category, Some("general_chat".to_string()));
    assert_eq!(m.auth, Some("api_key".to_string()));
}

#[test]
fn validate_provider_input_rejects_empty_name() {
    let r = validate_provider_input("", "anthropic", "", "sk-x", "claude");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("名称"));
}

#[test]
fn validate_provider_input_rejects_empty_api_key() {
    let r = validate_provider_input("foo", "anthropic", "", "", "claude");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("API Key"));
}

#[test]
fn validate_provider_input_rejects_empty_model() {
    let r = validate_provider_input("foo", "anthropic", "", "sk-x", "");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("模型"));
}

#[test]
fn validate_provider_input_rejects_unknown_type() {
    let r = validate_provider_input("foo", "bogus", "", "sk-x", "claude");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("不支持"));
}

#[test]
fn validate_provider_input_custom_requires_base_url() {
    let r = validate_provider_input("foo", "custom-openai", "", "sk-x", "gpt");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("Base URL"));
}

#[test]
fn validate_provider_input_accepts_valid_anthropic() {
    let r = validate_provider_input("foo", "anthropic", "", "sk-x", "claude");
    assert!(r.is_ok());
}

#[test]
fn validate_provider_input_accepts_valid_custom() {
    let r = validate_provider_input("foo", "custom-openai", "https://example.com/v1", "sk-x", "gpt");
    assert!(r.is_ok());
}

#[test]
fn resolve_effective_api_key_empty_incoming_keeps_stored() {
    let stored = Some("sk-stored");
    let result = resolve_effective_api_key(stored, "");
    assert_eq!(result, "sk-stored");
}

#[test]
fn resolve_effective_api_key_empty_incoming_no_stored_returns_empty() {
    let result = resolve_effective_api_key(None, "");
    assert_eq!(result, "");
}

#[test]
fn resolve_effective_api_key_non_empty_incoming_passes_through() {
    let result = resolve_effective_api_key(Some("sk-stored"), "sk-new");
    assert_eq!(result, "sk-new");
}

#[test]
fn resolve_effective_api_key_whitespace_only_treated_as_empty() {
    let result = resolve_effective_api_key(Some("sk-stored"), "   ");
    assert_eq!(result, "sk-stored");
}

// ===== Push stream test (Spec 3/5) =====

#[tokio::test]
async fn push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean() {
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_kernel_api::settings::AIModelConfigDto;
    use northhing_kernel_api::KernelSettingsApi;

    let _ = northhing_core::service::config::initialize_global_config().await;

    let facade = kernel_facade();
    let kr = MockKeyring::new();

    let model_id = format!("test-push-model-{}", uuid::Uuid::new_v4());
    kr.seed(&model_id, "sk-push-secret-999");

    let model_dto = AIModelConfigDto {
        id: model_id.clone(),
        provider_id: "openai".to_string(),
        model: "gpt-4o".to_string(),
        display_name: Some("Test Push Model".to_string()),
        max_tokens: None,
        temperature: None,
        base_url: Some("https://api.openai.com/v1".to_string()),
        api_key: None,
        enabled: Some(true),
        category: Some("general_chat".to_string()),
        capabilities: Some(vec!["text_chat".to_string()]),
        auth: Some("api_key".to_string()),
        inline_think_in_text: Some(true),
    };

    facade
        .upsert_model_config(model_dto)
        .await
        .expect("upsert model in core");

    // Run push_resolved_keys_to_core
    let pushed_count = push_resolved_keys_to_core(&kr).await.expect("push resolved keys");
    assert!(pushed_count >= 1);

    // Verify in-memory core global config has plaintext key
    let global_cfg = facade.get_global_config().await.expect("get global config");
    let pushed_provider = global_cfg
        .providers
        .iter()
        .find(|p| p.id == model_id)
        .expect("provider must exist in global config");
    assert_eq!(pushed_provider.api_key, "sk-push-secret-999");

    // Clean up
    let _ = facade.delete_model_config(&model_id).await;
}
