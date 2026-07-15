//! Re-export facade for `service::config` types.
//!
//! All concrete types now live in sibling files
//! (`app_shell`, `theme`, `editor`, `terminal`, `workspace`, `ai`, `runtime`, `events`).
//!
//! This facade provides:
//!
//! - `crate::service::config::types::XXConfig` paths (legacy 40 cross-crate sites)
//! - `crate::service::config::XXConfig` paths (via `mod.rs`'s `pub use types::*;`)
//! - `#[cfg(test)] mod tests` access to every sibling item via `super::*`

// Double re-export: preserve 40 cross-crate `types::XX` import paths.
pub use super::ai::*;
pub use super::app_shell::*;
pub use super::editor::*;
pub use super::events::*;
pub use super::runtime::*;
pub use super::terminal::*;
pub use super::theme::*;
pub use super::workspace::*;

#[cfg(test)]
mod shell_security_tests {
    use super::*;

    #[test]
    fn default_config_resolves_permissive_for_all_modes() {
        let cfg = ShellSecurityConfig::default();
        // Default: Permissive global, so all modes skip confirmation
        assert_eq!(cfg.confirmation_mode, ConfirmationMode::Permissive);
        assert!(cfg.should_skip_confirmation("agentic"));
        assert!(cfg.should_skip_confirmation("plan"));
        assert!(cfg.should_skip_confirmation("debug"));
        assert!(cfg.should_skip_confirmation("admin"));
        assert!(cfg.should_skip_confirmation("custom_mode"));
    }

    #[test]
    fn strict_global_default_makes_all_modes_strict() {
        let cfg = ShellSecurityConfig {
            confirmation_mode: ConfirmationMode::Strict,
            ..Default::default()
        };
        // All modes strict when global is Strict
        assert!(!cfg.should_skip_confirmation("admin"));
        assert!(!cfg.should_skip_confirmation("agentic"));
    }

    #[test]
    fn mode_override_wins_over_global_default() {
        let mut cfg = ShellSecurityConfig {
            confirmation_mode: ConfirmationMode::Strict,
            ..Default::default()
        };
        // Override agentic back to Permissive even though global is Strict
        cfg.mode_overrides
            .insert("agentic".to_string(), ConfirmationMode::Permissive);
        assert!(cfg.should_skip_confirmation("agentic"));
        // admin still strict (no override)
        assert!(!cfg.should_skip_confirmation("admin"));
    }

    #[test]
    fn mode_override_can_promote_coding_mode_to_strict() {
        let mut cfg = ShellSecurityConfig::default();
        cfg.mode_overrides.insert("debug".to_string(), ConfirmationMode::Strict);
        // debug promoted to strict via override
        assert!(!cfg.should_skip_confirmation("debug"));
        // agentic still permissive (global default)
        assert!(cfg.should_skip_confirmation("agentic"));
    }

    #[test]
    fn default_mode_policies_map_documented_modes() {
        let cfg = ShellSecurityConfig::default();
        // Default policies document known modes (informational, not used by resolve)
        assert_eq!(
            cfg.default_mode_policies.get("agentic"),
            Some(&ConfirmationMode::Permissive)
        );
        assert_eq!(
            cfg.default_mode_policies.get("admin"),
            None // admin not in default policies; uses global
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AIConfig, AIExperienceConfig, AIModelConfig, AppLoggingConfig, GlobalConfig, ModelExchangeTracingMode,
        ReasoningMode,
    };

    #[test]
    fn deserializes_compatibility_thinking_flag_into_reasoning_mode() {
        let config: AIModelConfig = serde_json::from_value(serde_json::json!({
            "id": "model_1",
            "name": "Provider",
            "provider": "openai",
            "model_name": "test-model",
            "base_url": "https://example.com/v1",
            "api_key": "key",
            "enabled": true,
            "enable_thinking_process": true
        }))
        .expect("legacy config should deserialize");

        assert_eq!(config.reasoning_mode, Some(ReasoningMode::Enabled));
        assert!(config.enable_thinking_process);
    }

    #[test]
    fn global_config_preserves_project_mcp_servers() {
        let config: GlobalConfig = serde_json::from_value(serde_json::json!({
            "project": {
                "mcp_servers": [
                    {
                        "id": "project-docs",
                        "name": "Project Docs",
                        "server_type": "local",
                        "command": "docs-mcp",
                        "args": []
                    }
                ]
            }
        }))
        .expect("project scoped MCP config should deserialize");

        assert_eq!(
            config
                .project
                .mcp_servers
                .as_ref()
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(1)
        );

        let serialized = serde_json::to_value(&config).expect("config should serialize");
        assert_eq!(serialized["project"]["mcp_servers"][0]["id"], "project-docs");
    }

    #[test]
    fn global_config_preserves_terminal_panel_position() {
        let config: GlobalConfig = serde_json::from_value(serde_json::json!({
            "terminal": {
                "terminal_panel_position": "bottom"
            }
        }))
        .expect("terminal panel position config should deserialize");

        assert_eq!(config.terminal.terminal_panel_position, "bottom");

        let serialized = serde_json::to_value(&config).expect("config should serialize");
        assert_eq!(serialized["terminal"]["terminal_panel_position"], "bottom");
    }

    #[test]
    fn defaults_agent_companion_pet_to_northhing() {
        let config: AIExperienceConfig =
            serde_json::from_value(serde_json::json!({})).expect("empty config should default");

        let pet = config
            .agent_companion_pet
            .as_ref()
            .expect("default companion pet should be present");
        assert_eq!(pet.id, "northhing");
        assert_eq!(pet.display_name, "northhing");
        assert_eq!(pet.package_path, "/agent-companion-pets/northhing");
        assert_eq!(pet.spritesheet_path, "/agent-companion-pets/northhing/spritesheet.webp");
    }

    #[test]
    fn preserves_selected_agent_companion_pet() {
        let config: AIExperienceConfig = serde_json::from_value(serde_json::json!({
            "enable_session_title_generation": true,
            "enable_welcome_panel_ai_analysis": false,
            "enable_visual_mode": false,
            "enable_agent_companion": true,
            "agent_companion_display_mode": "desktop",
            "agent_companion_pet": {
                "id": "boxcat",
                "displayName": "Boxcat",
                "description": "A tiny cat tucked inside a cardboard box for cozy coding sessions.",
                "source": "preset",
                "packagePath": "/agent-companion-pets/boxcat",
                "spritesheetPath": "/agent-companion-pets/boxcat/spritesheet.webp",
                "spritesheetMimeType": "image/webp"
            }
        }))
        .expect("AI experience config with selected companion pet should deserialize");

        let pet = config
            .agent_companion_pet
            .as_ref()
            .expect("selected companion pet should be retained");
        assert_eq!(pet.id, "boxcat");
        assert_eq!(pet.display_name, "Boxcat");
        assert_eq!(pet.package_path, "/agent-companion-pets/boxcat");
        assert_eq!(config.agent_companion_display_mode, "desktop");

        let serialized = serde_json::to_value(&config).expect("config should serialize");
        assert_eq!(serialized["agent_companion_pet"]["displayName"], "Boxcat");
        assert_eq!(
            serialized["agent_companion_pet"]["spritesheetPath"],
            "/agent-companion-pets/boxcat/spritesheet.webp"
        );
    }

    #[test]
    fn ai_experience_quick_actions_round_trip_through_global_config() {
        let config: GlobalConfig = serde_json::from_value(serde_json::json!({
            "app": {
                "language": "en-US",
                "auto_update": true,
                "telemetry": true,
                "startup_behavior": "default",
                "confirm_on_exit": true,
                "restore_windows": false,
                "zoom_level": 100,
                "sidebar": { "width": 260, "collapsed": false },
                "right_panel": { "width": 400, "collapsed": true },
                "notifications": {
                    "enabled": true,
                    "position": "top-right",
                    "duration": 4000,
                    "dialog_completion_notify": true,
                    "enable_startup_tips": true
                },
                "session_config": { "default_mode": "code" },
                "ai_experience": {
                    "enable_session_title_generation": true,
                    "enable_welcome_panel_ai_analysis": false,
                    "enable_visual_mode": false,
                    "enable_agent_companion": true,
                    "agent_companion_display_mode": "desktop",
                    "enable_workspace_search": false,
                    "quick_actions": [
                        {
                            "id": "custom_1",
                            "label": "Run tests",
                            "prompt": "Run the test suite",
                            "enabled": true
                        }
                    ]
                }
            }
        }))
        .expect("minimal app config with quick_actions should deserialize");

        let actions = &config.app.ai_experience.quick_actions;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "custom_1");
        assert_eq!(actions[0].label, "Run tests");

        let serialized = serde_json::to_value(&config).expect("config should serialize");
        assert_eq!(serialized["app"]["ai_experience"]["quick_actions"][0]["id"], "custom_1");
    }

    #[test]
    fn deserializes_compatibility_false_thinking_flag_into_default_reasoning_mode() {
        let config: AIModelConfig = serde_json::from_value(serde_json::json!({
            "id": "model_1",
            "name": "Provider",
            "provider": "openai",
            "model_name": "test-model",
            "base_url": "https://example.com/v1",
            "api_key": "key",
            "enabled": true,
            "enable_thinking_process": false
        }))
        .expect("legacy config should deserialize");

        assert_eq!(config.reasoning_mode, Some(ReasoningMode::Default));
        assert!(!config.enable_thinking_process);
    }

    #[test]
    fn serialization_omits_compatibility_thinking_flag() {
        let config: AIModelConfig = serde_json::from_value(serde_json::json!({
            "id": "model_1",
            "name": "Provider",
            "provider": "openai",
            "model_name": "test-model",
            "base_url": "https://example.com/v1",
            "api_key": "key",
            "enabled": true,
            "enable_thinking_process": true
        }))
        .expect("legacy config should deserialize");

        let value = serde_json::to_value(&config).expect("config should serialize");

        assert!(value.get("enable_thinking_process").is_none());
        assert_eq!(value.get("reasoning_mode").and_then(|v| v.as_str()), Some("enabled"));
    }

    #[test]
    fn default_model_config_enables_inline_think_in_text() {
        let config = AIModelConfig::default();
        assert!(config.inline_think_in_text);
    }

    #[test]
    fn deserializes_missing_inline_think_in_text_as_enabled() {
        let config: AIModelConfig = serde_json::from_value(serde_json::json!({
            "id": "model_1",
            "name": "Provider",
            "provider": "openai",
            "model_name": "test-model",
            "base_url": "https://example.com/v1",
            "api_key": "key",
            "enabled": true
        }))
        .expect("config without inline_think_in_text should deserialize");

        assert!(config.inline_think_in_text);
    }

    #[test]
    fn default_ai_config_uses_stream_timeouts() {
        let config = AIConfig::default();

        assert_eq!(config.stream_idle_timeout_secs, Some(45));
        assert_eq!(config.stream_ttft_timeout_secs, Some(30));
        assert_eq!(config.subagent_max_concurrency, 5);
        let review_team = config
            .review_teams
            .get("default")
            .expect("default review team config should exist");
        assert_eq!(review_team.reviewer_timeout_seconds, 3600);
        assert_eq!(review_team.judge_timeout_seconds, 2400);
        assert!(!review_team.auto_fix_enabled);
        assert_eq!(review_team.strategy_level, "normal");
        assert!(review_team.member_strategy_overrides.is_empty());
        assert_eq!(config.review_team_rate_limit_status, serde_json::json!({}));
        assert!(config.review_team_project_strategy_overrides.is_empty());
    }

    #[test]
    fn deserializes_missing_stream_idle_timeout_as_default() {
        let config: AIConfig = serde_json::from_value(serde_json::json!({
            "models": [],
            "agent_models": {},
            "func_agent_models": {},
            "default_models": {},
            "agent_profiles": {},
            "proxy": {
                "enabled": false,
                "url": ""
            }
        }))
        .expect("config without stream_idle_timeout_secs should deserialize");

        assert_eq!(config.stream_idle_timeout_secs, Some(45));
        assert_eq!(config.stream_ttft_timeout_secs, Some(30));
        assert_eq!(config.subagent_max_concurrency, 5);
        assert!(config.review_teams.contains_key("default"));
    }

    #[test]
    fn app_logging_defaults_to_sensitive_diagnostics_enabled() {
        let config: AppLoggingConfig = serde_json::from_value(serde_json::json!({
            "level": "trace"
        }))
        .expect("logging config without sensitive preference should deserialize");

        assert!(config.include_sensitive_diagnostics);
        assert_eq!(config.model_exchange_tracing.mode, ModelExchangeTracingMode::Off);
    }

    #[test]
    fn deserializes_explicit_subagent_max_concurrency() {
        let config: AIConfig = serde_json::from_value(serde_json::json!({
            "models": [],
            "agent_models": {},
            "func_agent_models": {},
            "default_models": {},
            "agent_profiles": {},
            "subagent_max_concurrency": 9,
            "proxy": {
                "enabled": false,
                "url": ""
            }
        }))
        .expect("config with subagent_max_concurrency should deserialize");

        assert_eq!(config.subagent_max_concurrency, 9);
    }

    #[test]
    fn deserializes_mode_profiles_with_null_entries() {
        let config: AIConfig = serde_json::from_value(serde_json::json!({
            "models": [],
            "agent_models": {},
            "func_agent_models": {},
            "default_models": {},
            "agent_profiles": {
                "Claw": null,
                "Cowork": {
                    "profile_id": "Cowork",
                    "removed_tools": ["shell"]
                }
            },
            "proxy": {
                "enabled": false,
                "url": ""
            }
        }))
        .expect("config with null mode config entries should deserialize");

        assert!(!config.agent_profiles.contains_key("Claw"));
        assert_eq!(
            config
                .agent_profiles
                .get("Cowork")
                .expect("non-null mode config should be retained")
                .removed_tools,
            vec!["shell".to_string()]
        );
    }

    #[test]
    fn deserializes_explicit_default_review_team_config() {
        let config: AIConfig = serde_json::from_value(serde_json::json!({
            "models": [],
            "agent_models": {},
            "func_agent_models": {},
            "default_models": {},
            "agent_profiles": {},
            "review_teams": {
                "default": {
                    "extra_subagent_ids": ["ExtraReviewer"],
                    "reviewer_timeout_seconds": 120,
                    "judge_timeout_seconds": 90,
                    "strategy_level": "deep",
                    "member_strategy_overrides": {
                        "ReviewSecurity": "quick",
                        "ExtraReviewer": "normal"
                    },
                    "auto_fix_enabled": false
                }
            },
            "proxy": {
                "enabled": false,
                "url": ""
            }
        }))
        .expect("config with review_teams should deserialize");

        let review_team = config
            .review_teams
            .get("default")
            .expect("default review team config should be retained");
        assert_eq!(review_team.extra_subagent_ids, vec!["ExtraReviewer".to_string()]);
        assert_eq!(review_team.reviewer_timeout_seconds, 120);
        assert_eq!(review_team.judge_timeout_seconds, 90);
        assert_eq!(review_team.strategy_level, "deep");
        assert_eq!(
            review_team.member_strategy_overrides.get("ReviewSecurity"),
            Some(&"quick".to_string())
        );
        assert_eq!(
            review_team.member_strategy_overrides.get("ExtraReviewer"),
            Some(&"normal".to_string())
        );
        assert!(!review_team.auto_fix_enabled);

        let serialized = serde_json::to_value(&config).expect("config should serialize");
        assert_eq!(serialized["review_teams"]["default"]["strategy_level"], "deep");
        assert_eq!(
            serialized["review_teams"]["default"]["member_strategy_overrides"]["ReviewSecurity"],
            "quick"
        );
    }

    #[test]
    fn review_team_auxiliary_config_is_not_stored_inside_review_team_map() {
        let config: AIConfig = serde_json::from_value(serde_json::json!({
            "models": [],
            "agent_models": {},
            "review_teams": {
                "default": {
                    "strategy_level": "normal"
                }
            },
            "review_team_rate_limit_status": {
                "remaining": 2
            },
            "review_team_project_strategy_overrides": {
                "workspace/repo": "quick"
            }
        }))
        .expect("review team auxiliary config should deserialize");

        assert!(config.review_teams.contains_key("default"));
        assert!(!config.review_teams.contains_key("rate_limit_status"));
        assert_eq!(config.review_team_rate_limit_status["remaining"], serde_json::json!(2));
        assert_eq!(
            config.review_team_project_strategy_overrides.get("workspace/repo"),
            Some(&"quick".to_string())
        );

        let serialized = serde_json::to_value(&config).expect("review team auxiliary config should serialize");
        assert!(serialized["review_teams"]["rate_limit_status"].is_null());
        assert_eq!(
            serialized["review_team_project_strategy_overrides"]["workspace/repo"],
            "quick"
        );
    }
}
