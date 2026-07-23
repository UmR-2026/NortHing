use super::*;
use std::collections::HashMap;

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
fn is_first_run_legacy_placeholders_only_still_first_run() {
    // Spec Q9=a: P0-B seeded 3 disabled placeholders should NOT count as
    // "real" providers. is_first_run() returns true so the welcome
    // screen still appears for users whose app.json only has the seeds.
    let mut s = AppSettings::default();
    s.providers.push(ProviderConfig {
        id: "anthropic-default".into(),
        name: "Anthropic Claude".into(),
        provider_type: ProviderType::Anthropic,
        base_url: String::new(),
        api_key: String::new(),
        model: "claude-sonnet-4-5".into(),
        enabled: false,
        created_at: 0,
        last_verified_at: None,
        last_verified_ok: None,
    });
    assert!(s.is_first_run(), "legacy placeholder should not block welcome");
    assert!(s.has_legacy_placeholders(), "should detect legacy");
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
fn skill_effective_precedence() {
    let mut s = SkillState {
        name: "memory".into(),
        global_enabled: true,
        workspace_overrides: HashMap::new(),
    };
    // Default: global on.
    assert!(s.effective_in(Path::new("/anywhere")));

    // Global off, no override → off.
    s.global_enabled = false;
    assert!(!s.effective_in(Path::new("/anywhere")));

    // Workspace override beats global.
    s.workspace_overrides.insert(PathBuf::from("/myproj"), true);
    assert!(s.effective_in(Path::new("/myproj")));
    assert!(!s.effective_in(Path::new("/elsewhere")));
}

#[test]
fn upsert_provider_replaces_by_id() {
    let mut s = AppSettings::default();
    let mut p = sample_provider();
    s.upsert_provider(p.clone());
    s.upsert_provider(p.clone());
    assert_eq!(s.providers.len(), 1);
    p.api_key = "sk-test".into();
    s.upsert_provider(p);
    assert_eq!(s.providers.len(), 1);
    assert_eq!(s.providers[0].api_key, "sk-test");
}

#[test]
fn fallback_provider_skips_self() {
    let mut s = AppSettings::default();
    let mut a = sample_provider();
    let mut b = sample_provider();
    // 2026-07-18 (D2c): make (name, base_url, api_key) distinct so the new
    // dedup logic does not collapse them — this test is about fallback
    // selection, not dedup.
    a.name = "a".to_string();
    b.name = "b".to_string();
    let b_id = b.id.clone();
    s.upsert_provider(a);
    s.upsert_provider(b);
    assert_eq!(s.providers.len(), 2);
    // Remove a; b should be the fallback.
    let a_id = s.providers[0].id.clone();
    s.remove_provider(&a_id);
    let fb = s.fallback_provider_for(&a_id);
    assert_eq!(fb.map(|p| p.id.clone()), Some(b_id));
}

#[test]
fn resolve_default_model_falls_back_when_provider_deleted() {
    let mut s = AppSettings::default();
    let a = sample_provider();
    let a_id = a.id.clone();
    s.upsert_provider(a.clone());
    s.default_model = Some(ModelRef {
        provider_id: a_id.clone(),
        model: a.model.clone(),
    });
    // Remove the default's provider.
    s.remove_provider(&a_id);
    // Should fall back to first enabled (none in this case).
    assert!(s.resolve_default_model().is_none());
}

#[test]
fn settings_json_roundtrip() {
    let mut s = AppSettings::default();
    s.upsert_provider(sample_provider());
    s.add_workspace(PathBuf::from("/tmp/proj"));
    let json = serde_json::to_string_pretty(&s).unwrap();
    let back: AppSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.providers.len(), 1);
    assert_eq!(back.workspaces.len(), 1);
}

// 2026-06-26 (Phase 4 fix): onboarding_completed serde default +
// roundtrip. Pre-existing app.json files lack the field and must
// deserialize to `false`; once set to `true` it round-trips cleanly.
#[test]
fn onboarding_completed_serde_default_false() {
    let full = serde_json::to_value(AppSettings::default()).expect("serialize default");
    let mut obj = full.as_object().expect("object").clone();
    obj.remove("onboarding_completed");
    let s: AppSettings = serde_json::from_value(serde_json::Value::Object(obj))
        .expect("deserialize without onboarding_completed");
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

// 2026-06-26 (Phase 5): Q6/Q7 session integrity validation tests.
// See `validate_session_integrity` in the `impl AppSettings` block
// above for the implementation rationale.

fn sample_session_provider() -> ProviderConfig {
    ProviderConfig::new("test-anthropic".to_string(), ProviderType::Anthropic)
}

#[test]
fn validate_session_integrity_detects_deleted_provider() {
    let mut s = AppSettings::default();
    let p = sample_session_provider();
    let p_id = p.id.clone();
    s.upsert_provider(p);
    // Add the workspace too so this test only checks Q6.
    s.add_workspace(PathBuf::from("/tmp/proj"));

    // Session references p_id + the workspace.
    let provider_lookup = |_sid: &str| -> Option<String> { Some(p_id.clone()) };
    let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/proj")) };

    // Before deletion: no issues.
    let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
    assert!(issues.is_empty(), "no issues when provider + workspace exist");

    // Delete the provider; expect the session to be flagged with Q6.
    s.remove_provider(&p_id);
    let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
    assert_eq!(issues.len(), 1, "expected exactly the Q6 issue");
    assert_eq!(issues[0].kind, "provider-deleted");
    assert_eq!(issues[0].session_id, "s1");
    assert_eq!(issues[0].related_id, p_id);
}

#[test]
fn validate_session_integrity_detects_removed_workspace() {
    let mut s = AppSettings::default();
    s.add_workspace(PathBuf::from("/tmp/exists"));

    // Session belongs to a workspace that's not in the list.
    let provider_lookup = |_sid: &str| -> Option<String> { None };
    let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/removed")) };

    let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].kind, "workspace-removed");
    assert_eq!(issues[0].related_id, "/tmp/removed");
}

#[test]
fn validate_session_integrity_reports_both_q6_and_q7_per_session() {
    // A session can be both: provider gone + workspace gone.
    let mut s = AppSettings::default();
    s.upsert_provider(sample_session_provider());

    let provider_lookup = |_sid: &str| -> Option<String> { Some("missing-provider".to_string()) };
    let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/missing")) };

    let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
    assert_eq!(issues.len(), 2);
    let kinds: Vec<&str> = issues.iter().map(|i| i.kind.as_str()).collect();
    assert!(kinds.contains(&"provider-deleted"));
    assert!(kinds.contains(&"workspace-removed"));
}

#[test]
fn validate_session_integrity_empty_session_list_is_noop() {
    let s = AppSettings::default();
    let issues = s.validate_session_integrity(std::iter::empty::<String>(), |_| None, |_| None);
    assert!(issues.is_empty());
}

/// Integration test: simulate the spec's "完整欢迎流程 + 添加
/// provider + 创建 session + 删除 provider" flow at the
/// AppSettings level. After the sequence, `validate_session_integrity`
/// must report the Q6 (provider-deleted) issue for the session
/// that referenced the now-gone provider.
#[test]
fn integration_welcome_provider_session_delete_provider() {
    use std::collections::HashMap;

    // Step 1: empty settings → first-run flag set.
    let mut s = AppSettings::default();
    assert!(s.is_first_run(), "empty settings is first run");

    // Step 2: user adds a workspace (welcome step 1).
    s.add_workspace(PathBuf::from("/tmp/proj"));
    s.set_current_workspace(Some(&PathBuf::from("/tmp/proj")));
    assert!(!s.is_first_run(), "after workspace, not first run");

    // Step 3: user adds a provider (welcome step 2).
    let provider = sample_provider();
    let provider_id = provider.id.clone();
    s.upsert_provider(provider);
    s.default_model = Some(ModelRef {
        provider_id: provider_id.clone(),
        model: "claude-sonnet-4-5".to_string(),
    });

    // Step 4: user creates a session using the provider.
    let session_id = "sess-1".to_string();
    let mut session_provider_lookup = HashMap::new();
    session_provider_lookup.insert(session_id.clone(), provider_id.clone());
    let mut session_workspace_lookup = HashMap::new();
    session_workspace_lookup.insert(session_id.clone(), PathBuf::from("/tmp/proj"));
    let provider_lookup = |sid: &str| -> Option<String> { session_provider_lookup.get(sid).cloned() };
    let workspace_lookup = |sid: &str| -> Option<PathBuf> { session_workspace_lookup.get(sid).cloned() };

    // No issues yet.
    let issues = s.validate_session_integrity(vec![session_id.clone()], &provider_lookup, &workspace_lookup);
    assert!(issues.is_empty(), "all healthy before delete");

    // Step 5: user deletes the provider in Settings.
    s.remove_provider(&provider_id);

    // Now integrity should flag the session.
    let issues = s.validate_session_integrity(vec![session_id.clone()], &provider_lookup, &workspace_lookup);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].session_id, session_id);
    assert_eq!(issues[0].kind, "provider-deleted");
    assert_eq!(issues[0].related_id, provider_id);

    // And the default model should fall back to nothing.
    assert!(s.resolve_default_model().is_none());
}

// ===== Core sync helper tests =====

#[test]
fn provider_wire_format_mapping() {
    assert_eq!(provider_wire_format(&ProviderType::Anthropic), "anthropic");
    assert_eq!(provider_wire_format(&ProviderType::Openai), "openai");
    assert_eq!(provider_wire_format(&ProviderType::Gemini), "gemini");
    assert_eq!(
        provider_wire_format(&ProviderType::CustomOpenaiCompatible),
        "openai"
    );
    assert_eq!(
        provider_wire_format(&ProviderType::CustomAnthropicCompatible),
        "anthropic"
    );
}

#[test]
fn provider_to_ai_model_config_fields() {
    let p = ProviderConfig::new("我的 Anthropic".into(), ProviderType::Anthropic);
    let m = provider_to_ai_model_config(&p);
    assert_eq!(m.id, p.id);
    assert_eq!(m.name, "我的 Anthropic");
    assert_eq!(m.provider, "anthropic");
    assert_eq!(m.model_name, p.model);
    assert_eq!(m.api_key, p.api_key);
    assert_eq!(m.enabled, p.enabled);
    assert!(m.base_url.contains("anthropic"));
    assert_eq!(m.category, northhing_core::service::config::ModelCategory::GeneralChat);
    assert_eq!(m.auth, northhing_core::service::config::AuthConfig::ApiKey);
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
    let r = validate_provider_input(
        "foo",
        "custom-openai",
        "https://example.com/v1",
        "sk-x",
        "gpt",
    );
    assert!(r.is_ok());
}

// ===== 2026-07-18 (D2c): upsert dedup + default-model auto-set tests =====

fn provider_with_fields(
    id: &str,
    name: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    enabled: bool,
) -> ProviderConfig {
    ProviderConfig {
        id: id.to_string(),
        name: name.to_string(),
        provider_type: if base_url.contains("anthropic") {
            ProviderType::Anthropic
        } else {
            ProviderType::Openai
        },
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        model: model.to_string(),
        enabled,
        created_at: 0,
        last_verified_at: None,
        last_verified_ok: None,
    }
}

#[test]
fn upsert_provider_dedup_by_name_base_url_api_key_keeps_original_id() {
    let mut s = AppSettings::default();
    // First upsert: empty id → push new (gets a fresh UUID).
    let first = provider_with_fields("", "foo", "https://x.com/v1", "sk-same", "gpt", true);
    s.upsert_provider(first);
    assert_eq!(s.providers.len(), 1);
    let original_id = s.providers[0].id.clone();

    // Second upsert: different id but same (name, base_url, api_key) →
    // should replace in place and KEEP the original id.
    let second = provider_with_fields(
        "totally-different-id",
        "foo",
        "https://x.com/v1",
        "sk-same",
        "gpt",
        true,
    );
    s.upsert_provider(second);
    assert_eq!(s.providers.len(), 1, "must not duplicate");
    assert_eq!(
        s.providers[0].id, original_id,
        "must keep the original id to preserve session references"
    );
}

#[test]
fn upsert_provider_first_enabled_auto_sets_default_model() {
    let mut s = AppSettings::default();
    assert!(s.default_model.is_none());
    let p = provider_with_fields("", "foo", "https://x.com/v1", "sk", "gpt", true);
    s.upsert_provider(p);
    assert_eq!(s.providers.len(), 1);
    let dm = s.default_model.expect("default_model should be auto-set");
    assert_eq!(dm.provider_id, s.providers[0].id);
    assert_eq!(dm.model, "gpt");
}

#[test]
fn upsert_provider_does_not_overwrite_existing_default_model() {
    let mut s = AppSettings::default();
    let first = provider_with_fields("id-first", "first", "https://a.com/v1", "sk1", "m1", true);
    s.upsert_provider(first);
    let first_dm = s.default_model.clone().unwrap();

    // Second enabled provider → default_model must stay pointing at first.
    let second = provider_with_fields("id-second", "second", "https://b.com/v1", "sk2", "m2", true);
    s.upsert_provider(second);
    assert_eq!(s.providers.len(), 2);
    let dm = s.default_model.unwrap();
    assert_eq!(dm.provider_id, first_dm.provider_id);
    assert_eq!(dm.model, first_dm.model);
}

#[test]
fn dedup_providers_on_load_drops_duplicates_keeps_first() {
    let mut s = AppSettings::default();
    // Two identical providers (same name/type/base_url/api_key/model) but
    // different ids → after dedup only the first remains.
    let a = provider_with_fields("id-a", "foo", "https://x.com/v1", "sk", "gpt", true);
    let b = provider_with_fields("id-b", "foo", "https://x.com/v1", "sk", "gpt", true);
    let c = provider_with_fields("id-c", "bar", "https://y.com/v1", "sk", "gpt", true);
    s.providers = vec![a, b, c];
    let dropped = dedup_providers_on_load(&mut s);
    assert_eq!(dropped, 1);
    assert_eq!(s.providers.len(), 2);
    assert_eq!(s.providers[0].id, "id-a", "first of group kept");
    assert_eq!(s.providers[1].id, "id-c");
}

#[test]
fn dedup_providers_on_load_repoints_default_model_when_dropped() {
    let mut s = AppSettings::default();
    let a = provider_with_fields("id-a", "foo", "https://x.com/v1", "sk", "gpt", true);
    let b = provider_with_fields("id-b", "foo", "https://x.com/v1", "sk", "gpt", true);
    s.providers = vec![a, b];
    // default_model points at id-b (the one that will be dropped).
    s.default_model = Some(ModelRef {
        provider_id: "id-b".to_string(),
        model: "gpt".to_string(),
    });
    let dropped = dedup_providers_on_load(&mut s);
    assert_eq!(dropped, 1);
    assert_eq!(s.providers.len(), 1);
    // After dedup, default_model should point at the kept entry (id-a).
    let dm = s.default_model.expect("default_model should be re-pointed");
    assert_eq!(dm.provider_id, "id-a");
}

// ===== 2026-07-18 (D2d): compute_stale_core_model_ids tests =====

#[test]
fn compute_stale_empty_existing_returns_empty() {
    let providers = vec![provider_with_fields("id-a", "a", "https://a.com/v1", "sk", "gpt", true)];
    let stale = compute_stale_core_model_ids(&[], &providers);
    assert!(stale.is_empty(), "no existing ids → nothing stale");
}

#[test]
fn compute_stale_partial_overlap_returns_only_extra() {
    let providers = vec![provider_with_fields("id-a", "a", "https://a.com/v1", "sk", "gpt", true)];
    let existing = vec!["id-a".to_string(), "id-b".to_string(), "id-c".to_string()];
    let stale = compute_stale_core_model_ids(&existing, &providers);
    assert_eq!(stale.len(), 2);
    assert!(stale.contains(&"id-b".to_string()));
    assert!(stale.contains(&"id-c".to_string()));
    assert!(!stale.contains(&"id-a".to_string()));
}

#[test]
fn compute_stale_all_stale_when_no_providers() {
    let existing = vec!["id-a".to_string(), "id-b".to_string()];
    let stale = compute_stale_core_model_ids(&existing, &[]);
    assert_eq!(stale.len(), 2);
    assert!(stale.contains(&"id-a".to_string()));
    assert!(stale.contains(&"id-b".to_string()));
}

#[test]
fn compute_stale_no_stale_when_all_match() {
    let providers = vec![
        provider_with_fields("id-a", "a", "https://a.com/v1", "sk1", "gpt", true),
        provider_with_fields("id-b", "b", "https://b.com/v1", "sk2", "claude", true),
    ];
    let existing = vec!["id-a".to_string(), "id-b".to_string()];
    let stale = compute_stale_core_model_ids(&existing, &providers);
    assert!(stale.is_empty(), "all existing ids matched → nothing stale");
}

// ===== 2026-07-18 (D2e): resolve_effective_api_key tests =====

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

// ===== 2026-07-18 (D2f): desired_primary_model_id tests =====

#[test]
fn desired_primary_model_id_returns_id_when_default_points_at_enabled_provider() {
    let mut s = AppSettings::default();
    let p = provider_with_fields("pid-123", "foo", "https://a.com/v1", "sk", "gpt", true);
    s.upsert_provider(p);
    s.default_model = Some(ModelRef {
        provider_id: "pid-123".to_string(),
        model: "gpt".to_string(),
    });
    assert_eq!(desired_primary_model_id(&s), Some("pid-123".to_string()));
}

#[test]
fn desired_primary_model_id_returns_none_when_provider_disabled() {
    let mut s = AppSettings::default();
    let p = provider_with_fields("pid-123", "foo", "https://a.com/v1", "sk", "gpt", false);
    s.upsert_provider(p);
    s.default_model = Some(ModelRef {
        provider_id: "pid-123".to_string(),
        model: "gpt".to_string(),
    });
    assert!(desired_primary_model_id(&s).is_none());
}

#[test]
fn desired_primary_model_id_returns_none_when_provider_not_found() {
    let mut s = AppSettings::default();
    // No providers at all.
    s.default_model = Some(ModelRef {
        provider_id: "nonexistent".to_string(),
        model: "gpt".to_string(),
    });
    assert!(desired_primary_model_id(&s).is_none());
}

#[test]
fn desired_primary_model_id_returns_none_when_default_model_is_none() {
    let s = AppSettings::default();
    assert!(s.default_model.is_none());
    assert!(desired_primary_model_id(&s).is_none());
}
