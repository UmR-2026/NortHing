// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W9-7 (2026-08-29) — Real-data cards for the settings "Its Self" column.
//
// The left column (它的自我) has four cards: sediment / chronicles /
// identity / axioms. This module renders them against real kernel data
// (facts, skills, sessions, default provider) instead of hardcoded mock
// text. It also exposes `persist_display_mode` so the right-column display
// toggles can write to AppSettings without leaking IO into `pages_settings`.

use dioxus::prelude::*;

use northhing_kernel_api::session::{SessionSummaryDto, WorkspaceSessionsDto};

use super::api;
use super::i18n::{keys, LocalePack};
use crate::app_state::settings::update_app_settings;

/// Maximum number of bar segments rendered in the sediment progress bar.
const SEDIMENT_SEG_MAX: usize = 5;
/// Minimum updated_at (ms since UNIX_EPOCH) treated as "no real data" —
/// guards against legacy fixtures that write `0` instead of an absent value.
const EPOCH_FLOOR_MS: i64 = 946_684_800_000; // 2000-01-01T00:00:00Z

// ===== AppSettings persistence helper =====

/// Persists display-mode toggles to AppSettings. Each argument is `Some` to
/// write or `None` to leave unchanged. Returns the first-line truncated
/// user-facing error on failure (empty string on success).
pub async fn persist_display_mode(breath: Option<bool>, dual_optics: Option<bool>) -> Result<(), String> {
    update_app_settings(|s| {
        if let Some(b) = breath {
            s.display_breath = b;
        }
        if let Some(d) = dual_optics {
            s.display_dual_optics = d;
        }
        Ok(())
    })
    .await
    .map_err(|e| {
        e.to_string()
            .lines()
            .next()
            .unwrap_or("保存失败")
            .trim()
            .to_string()
    })
}

// ===== Props =====

/// Props for the [`SelfColumn`] component. Each fold signal is owned by the
/// parent so the global fold-all button can toggle them in lockstep.
#[derive(Props, Clone, PartialEq)]
pub struct SelfColumnProps {
    pub locale: std::rc::Rc<LocalePack>,
    pub folded_sediment: Signal<bool>,
    pub folded_chronicles: Signal<bool>,
    pub folded_identity: Signal<bool>,
    pub folded_axioms: Signal<bool>,
}

// ===== SelfColumn =====

/// Renders the four "Its Self" cards against real data. Owns its own data
/// signals + load future; the parent only owns the fold signals.
#[component]
pub fn SelfColumn(props: SelfColumnProps) -> Element {
    let SelfColumnProps {
        locale,
        mut folded_sediment,
        mut folded_chronicles,
        mut folded_identity,
        mut folded_axioms,
    } = props;

    // Sediment
    let facts_count = use_signal(|| None::<usize>);
    let skills_count = use_signal(|| None::<usize>);
    let sediment_err = use_signal(|| None::<String>);

    // Chronicles
    let session_genesis = use_signal(|| None::<SessionSummaryDto>);
    let session_event = use_signal(|| None::<SessionSummaryDto>);
    let chronicles_err = use_signal(|| None::<String>);

    // Identity (default provider's display_name)
    let identity_name = use_signal(|| None::<String>);
    let identity_err = use_signal(|| None::<String>);

    // Load all four cards' data on mount. Per spec, no live subscription —
    // a fresh page open is the sync trigger.
    use_future(move || {
        let mut facts_count = facts_count;
        let mut skills_count = skills_count;
        let mut sediment_err = sediment_err;
        let mut session_genesis = session_genesis;
        let mut session_event = session_event;
        let mut chronicles_err = chronicles_err;
        let mut identity_name = identity_name;
        let mut identity_err = identity_err;
        async move {
            // Sediment: facts + skills counts
            match api::list_facts(None).await {
                Ok(facts) => facts_count.set(Some(facts.len())),
                Err(e) => sediment_err.set(Some(err_first_line(&e.to_string()))),
            }
            match api::list_skills().await {
                Ok(skills) => skills_count.set(Some(skills.len())),
                Err(e) => {
                    // Combine with facts error so the card shows both if both fail.
                    let prev = sediment_err().unwrap_or_default();
                    let line = err_first_line(&e.to_string());
                    sediment_err.set(Some(if prev.is_empty() { line } else { format!("{prev} | {line}") }));
                }
            }

            // Chronicles: oldest + newest by updated_at, skipping subagents.
            match api::list_sessions_all_workspaces().await {
                Ok(groups) => {
                    let (g, e) = pick_genesis_and_event(&groups);
                    session_genesis.set(g);
                    session_event.set(e);
                }
                Err(e) => chronicles_err.set(Some(err_first_line(&e.to_string()))),
            }

            // Identity: default provider's model.display_name (set during onboarding).
            match api::get_global_config().await {
                Ok(gcfg) => {
                    let default_id = gcfg.default_provider_id.clone();
                    match api::list_model_configs().await {
                        Ok(models) => {
                            let name = default_id
                                .as_ref()
                                .and_then(|id| models.iter().find(|m| &m.id == id))
                                .and_then(|m| m.display_name.clone())
                                .filter(|n| !n.trim().is_empty());
                            identity_name.set(name);
                        }
                        Err(e) => identity_err.set(Some(err_first_line(&e.to_string()))),
                    }
                }
                Err(e) => identity_err.set(Some(err_first_line(&e.to_string()))),
            }
        }
    });

    rsx! {
        // Card 1: 沉积记忆 SEDIMENT
        div {
            class: if folded_sediment() { "mod is-folded" } else { "mod" },
            div {
                class: "side-title w2-pin",
                onclick: move |_| { folded_sediment.toggle(); },
                "{locale.t(keys::SETTINGS_SECTION_SEDIMENT_TITLE)} "
                em { "{locale.t(keys::SETTINGS_SECTION_SEDIMENT_EM)}" }
                span { class: "fold-caret", if folded_sediment() { "▸" } else { "▾" } }
            }
            div { class: "w2-scroll",
                SedimentBody {
                    locale: locale.clone(),
                    facts_count: facts_count(),
                    skills_count: skills_count(),
                    error: sediment_err(),
                }
                div { class: "seg-note", "{locale.t(keys::SETTINGS_SEDIMENT_FOOT)}" }
            }
        }

        // Card 2: 编年史 CHRONICLES
        div {
            class: if folded_chronicles() { "mod is-folded" } else { "mod" },
            div {
                class: "side-title w2-pin",
                onclick: move |_| { folded_chronicles.toggle(); },
                "{locale.t(keys::SETTINGS_SECTION_CHRONICLES_TITLE)} "
                em { "{locale.t(keys::SETTINGS_SECTION_CHRONICLES_EM)}" }
                span { class: "fold-caret", if folded_chronicles() { "▸" } else { "▾" } }
            }
            div { class: "w2-scroll",
                if let Some(err) = chronicles_err() {
                    div { class: "row readonly", style: "color:var(--danger);", "{err}" }
                }
                if let Some(g) = session_genesis() {
                    div { class: "row readonly",
                        "Genesis"
                        span { class: "row-meta", "{chronicle_label(&g)}" }
                    }
                }
                if let Some(e) = session_event() {
                    div { class: "row readonly",
                        "Event"
                        span { class: "row-meta", "{chronicle_label(&e)}" }
                    }
                }
                if session_genesis().is_none() && session_event().is_none() && chronicles_err().is_none() {
                    div { class: "row readonly", "暂无会话记录" }
                }
            }
        }

        // Card 3: 身份 IDENTITY
        div {
            class: if folded_identity() { "mod is-folded" } else { "mod" },
            div {
                class: "side-title w2-pin",
                onclick: move |_| { folded_identity.toggle(); },
                "{locale.t(keys::SETTINGS_SECTION_IDENTITY_TITLE)} "
                em { "{locale.t(keys::SETTINGS_SECTION_IDENTITY_EM)}" }
                span { class: "fold-caret", if folded_identity() { "▸" } else { "▾" } }
            }
            div { class: "w2-scroll",
                if let Some(err) = identity_err() {
                    div { class: "row readonly", style: "color:var(--danger);", "名讳 加载失败: {err}" }
                }
                match identity_name() {
                    Some(name) => rsx! {
                        div { class: "row readonly",
                            "名讳"
                            span { class: "row-meta font-agent", "{name}" }
                        }
                    },
                    None => rsx! {
                        div { class: "row readonly",
                            "名讳"
                            span { class: "row-meta", "尚未在 onboarding 时命名" }
                        }
                    },
                }
                div { class: "row readonly",
                    "位格"
                    span { class: "row-meta", "未配置" }
                }
                if identity_err().is_some() {
                    div { class: "row readonly", style: "font-size:11px;color:var(--faint);",
                        "注：位格尚未提供独立数据通路。"
                    }
                }
            }
        }

        // Card 4: 准则 AXIOMS
        div {
            class: if folded_axioms() { "mod is-folded" } else { "mod" },
            div {
                class: "side-title w2-pin",
                onclick: move |_| { folded_axioms.toggle(); },
                "{locale.t(keys::SETTINGS_SECTION_AXIOMS_TITLE)} "
                em { "{locale.t(keys::SETTINGS_SECTION_AXIOMS_EM)}" }
                span { class: "fold-caret", if folded_axioms() { "▸" } else { "▾" } }
            }
            div { class: "w2-scroll",
                div { class: "row readonly",
                    "# 准则为产品设计原则，存储于 docs/，非用户数据。"
                }
                div { class: "row readonly",
                    "# 当前无配置入口。"
                }
            }
        }
    }
}

// ===== Sub-card body =====

#[derive(Props, Clone, PartialEq)]
struct SedimentBodyProps {
    locale: std::rc::Rc<LocalePack>,
    facts_count: Option<usize>,
    skills_count: Option<usize>,
    error: Option<String>,
}

#[component]
fn SedimentBody(props: SedimentBodyProps) -> Element {
    let SedimentBodyProps { locale: _, facts_count, skills_count, error } = props;
    let facts = facts_count.unwrap_or(0);
    let skills = skills_count.unwrap_or(0);
    let total = facts + skills;
    let segments_on = sediment_segments_on(total);

    if let Some(err) = error {
        return rsx! {
            div { class: "row readonly", style: "color:var(--danger);", "{err}" }
        };
    }

    rsx! {
        div { class: "row readonly",
            "记忆条目"
            span { class: "row-meta", "{facts}" }
        }
        div { class: "row readonly",
            "技能条目"
            span { class: "row-meta", "{skills}" }
        }
        div { class: "row readonly",
            "累计"
            span { class: "row-meta", "{total}" }
        }
        div { class: "seg-bar",
            for i in 0..SEDIMENT_SEG_MAX {
                div { key: "{i}", class: if i < segments_on { "seg on" } else { "seg" } }
            }
        }
    }
}

// ===== Pure helpers (unit-tested) =====

fn err_first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

/// Renders `updated_at` (millis) as `YYYY.MM`. Returns `—` when the timestamp
/// is missing or below the epoch floor (legacy fixture sentinel).
fn chronicle_label(s: &SessionSummaryDto) -> String {
    if s.updated_at < EPOCH_FLOOR_MS {
        return "—".to_string();
    }
    let secs = s.updated_at / 1000;
    let nanos = ((s.updated_at.rem_euclid(1000)) as u32) * 1_000_000;
    if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
        dt.format("%Y.%m").to_string()
    } else {
        "—".to_string()
    }
}

/// Filters out subagent sessions and returns the oldest + newest by
/// `updated_at`. Either side may be `None` if no user-visible session exists.
fn pick_genesis_and_event(
    groups: &[WorkspaceSessionsDto],
) -> (Option<SessionSummaryDto>, Option<SessionSummaryDto>) {
    let mut all: Vec<&SessionSummaryDto> = groups
        .iter()
        .flat_map(|g| g.sessions.iter())
        .filter(|s| s.parent_session_id.is_none())
        .collect();
    if all.is_empty() {
        return (None, None);
    }
    all.sort_by_key(|s| s.updated_at);
    let genesis = all.first().map(|s| (*s).clone());
    let event = all.last().map(|s| (*s).clone());
    (genesis, event)
}

/// Maps a count of (facts + skills) into how many of the 5 bar segments are
/// lit. Empty (`0`) = 0 segments; any non-zero count = 1 segment, capped at 5.
fn sediment_segments_on(total: usize) -> usize {
    if total == 0 {
        0
    } else {
        total.min(SEDIMENT_SEG_MAX)
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::session::SessionStatusDto;

    fn fixture_session(id: &str, updated_at: i64, parent: Option<&str>) -> SessionSummaryDto {
        SessionSummaryDto {
            id: id.to_string(),
            name: id.to_string(),
            updated_at,
            status: SessionStatusDto::Active,
            parent_session_id: parent.map(|s| s.to_string()),
            state: None,
        }
    }

    #[test]
    fn chronicle_label_renders_year_month() {
        // 2026-08-15T00:00:00Z in millis
        let ts: i64 = 1_785_753_600_000;
        let s = fixture_session("a", ts, None);
        assert_eq!(chronicle_label(&s), "2026.08");
    }

    #[test]
    fn chronicle_label_handles_zero_and_legacy_floor() {
        let z = fixture_session("z", 0, None);
        assert_eq!(chronicle_label(&z), "—");
        let floor = fixture_session("f", EPOCH_FLOOR_MS, None);
        assert_eq!(chronicle_label(&floor), "2000.01");
    }

    #[test]
    fn pick_genesis_and_event_excludes_subagents() {
        let groups = vec![WorkspaceSessionsDto {
            workspace_path: "/ws/a".to_string(),
            sessions: vec![
                fixture_session("genesis", 1_700_000_000_000, None),
                fixture_session("event", 1_800_000_000_000, None),
                fixture_session("sub", 1_750_000_000_000, Some("event")),
            ],
        }];
        let (g, e) = pick_genesis_and_event(&groups);
        assert_eq!(g.unwrap().id, "genesis");
        assert_eq!(e.unwrap().id, "event");
    }

    #[test]
    fn pick_genesis_and_event_empty_returns_none_pair() {
        let groups: Vec<WorkspaceSessionsDto> = vec![];
        let (g, e) = pick_genesis_and_event(&groups);
        assert!(g.is_none() && e.is_none());
    }

    #[test]
    fn pick_genesis_and_event_all_subagents_returns_none_pair() {
        let groups = vec![WorkspaceSessionsDto {
            workspace_path: "/ws/a".to_string(),
            sessions: vec![fixture_session("sub", 1_700_000_000_000, Some("parent"))],
        }];
        let (g, e) = pick_genesis_and_event(&groups);
        assert!(g.is_none() && e.is_none());
    }

    #[test]
    fn sediment_segments_on_zero_is_zero_otherwise_capped_at_five() {
        assert_eq!(sediment_segments_on(0), 0);
        assert_eq!(sediment_segments_on(1), 1);
        assert_eq!(sediment_segments_on(4), 4);
        assert_eq!(sediment_segments_on(5), 5);
        assert_eq!(sediment_segments_on(99), 5);
    }

    #[test]
    fn err_first_line_trims_whitespace() {
        assert_eq!(err_first_line("  oops  \nnext"), "oops");
        assert_eq!(err_first_line(""), "");
        assert_eq!(err_first_line("   "), "");
    }
}
