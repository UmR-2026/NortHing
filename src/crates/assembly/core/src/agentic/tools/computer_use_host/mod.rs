// Re-export optimizer types so downstream crates can import from computer_use_host.
pub use crate::agentic::tools::computer_use_optimizer::{ActionRecord, LoopDetectionResult};

#[path = "ch_actions.rs"]
mod ch_actions;
#[path = "ch_dispatch.rs"]
mod ch_dispatch;
#[path = "ch_platform.rs"]
mod ch_platform;
#[path = "ch_state.rs"]
mod ch_state;
#[path = "ch_types.rs"]
mod ch_types;

pub use ch_actions::*;
pub use ch_dispatch::*;
pub use ch_platform::*;
pub use ch_state::*;
pub use ch_types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_state_serializes_expected_shape() {
        let state = ComputerUseInteractionState {
            click_ready: false,
            enter_ready: true,
            requires_fresh_screenshot_before_click: true,
            requires_fresh_screenshot_before_enter: false,
            recommend_screenshot_to_verify_last_action: true,
            last_screenshot_kind: Some(ComputerUseInteractionScreenshotKind::FullDisplay),
            last_mutation: Some(ComputerUseLastMutationKind::Screenshot),
            recommended_next_action: Some("screenshot_navigate_quadrant".to_string()),
            displays: vec![],
            active_display_id: None,
        };

        let value = serde_json::to_value(&state).expect("serialize interaction state");

        assert_eq!(value["click_ready"], serde_json::json!(false));
        assert_eq!(value["enter_ready"], serde_json::json!(true));
        assert_eq!(value["requires_fresh_screenshot_before_click"], serde_json::json!(true));
        assert_eq!(
            value["requires_fresh_screenshot_before_enter"],
            serde_json::json!(false)
        );
        assert_eq!(value["last_screenshot_kind"], serde_json::json!("full_display"));
        assert_eq!(value["last_mutation"], serde_json::json!("screenshot"));
        assert_eq!(
            value["recommended_next_action"],
            serde_json::json!("screenshot_navigate_quadrant")
        );
        assert_eq!(
            value["recommend_screenshot_to_verify_last_action"],
            serde_json::json!(true)
        );
    }

    #[test]
    fn app_selector_constructors_populate_only_one_field() {
        let by_name = AppSelector::by_name("Safari");
        assert_eq!(by_name.name.as_deref(), Some("Safari"));
        assert!(by_name.bundle_id.is_none() && by_name.pid.is_none());
        assert!(!by_name.is_empty());

        let empty = AppSelector::default();
        assert!(empty.is_empty());
    }

    #[test]
    fn click_target_serializes_with_kind_tag() {
        let xy = ClickTarget::ScreenXy { x: 10.5, y: 20.0 };
        let v = serde_json::to_value(&xy).expect("serialize ScreenXy");
        assert_eq!(v["kind"], "screen_xy");
        assert_eq!(v["x"], serde_json::json!(10.5));

        let image_xy = ClickTarget::ImageXy {
            x: 100,
            y: 200,
            screenshot_id: Some("shot_1".to_string()),
        };
        let v = serde_json::to_value(&image_xy).expect("serialize ImageXy");
        assert_eq!(v["kind"], "image_xy");
        assert_eq!(v["x"], serde_json::json!(100));

        let grid = ClickTarget::ImageGrid {
            x0: 10,
            y0: 20,
            width: 300,
            height: 300,
            rows: 15,
            cols: 15,
            row: 7,
            col: 7,
            intersections: true,
            screenshot_id: Some("shot_1".to_string()),
        };
        let v = serde_json::to_value(&grid).expect("serialize ImageGrid");
        assert_eq!(v["kind"], "image_grid");
        assert_eq!(v["x0"], serde_json::json!(10));
        assert_eq!(v["intersections"], serde_json::json!(true));

        let visual_grid = ClickTarget::VisualGrid {
            rows: 15,
            cols: 15,
            row: 7,
            col: 7,
            intersections: true,
            wait_ms_after_detection: None,
        };
        let v = serde_json::to_value(&visual_grid).expect("serialize VisualGrid");
        assert_eq!(v["kind"], "visual_grid");

        let node = ClickTarget::NodeIdx { idx: 7 };
        let v = serde_json::to_value(&node).expect("serialize NodeIdx");
        assert_eq!(v["kind"], "node_idx");
        assert_eq!(v["idx"], serde_json::json!(7));

        let round_trip: ClickTarget = serde_json::from_value(v).expect("deserialize node_idx click target");
        assert_eq!(round_trip, ClickTarget::NodeIdx { idx: 7 });
    }

    #[test]
    fn app_click_params_apply_defaults_on_deserialize() {
        let json = serde_json::json!({
            "app": { "name": "Safari" },
            "target": { "kind": "node_idx", "idx": 3 },
        });
        let parsed: AppClickParams = serde_json::from_value(json).expect("deserialize minimal AppClickParams");
        assert_eq!(parsed.click_count, 1);
        assert_eq!(parsed.mouse_button, "left");
        assert!(parsed.modifier_keys.is_empty());
        assert_eq!(parsed.wait_ms_after, None);
        assert_eq!(parsed.app.name.as_deref(), Some("Safari"));
        assert_eq!(parsed.target, ClickTarget::NodeIdx { idx: 3 });
    }

    #[test]
    fn interactive_view_opts_apply_defaults_on_minimal_json() {
        let parsed: InteractiveViewOpts =
            serde_json::from_value(serde_json::json!({})).expect("deserialize empty opts");
        assert!(parsed.focus_window_only);
        assert!(parsed.annotate_screenshot);
        assert!(parsed.include_tree_text);
        assert_eq!(parsed.max_elements, None);
    }

    #[test]
    fn interactive_view_round_trips() {
        let view = InteractiveView {
            app: AppInfo {
                name: "Safari".into(),
                bundle_id: Some("com.apple.Safari".into()),
                pid: Some(123),
                running: true,
                last_used_ms: None,
                launch_count: 0,
            },
            window_title: Some("Apple".into()),
            elements: vec![InteractiveElement {
                i: 0,
                node_idx: 17,
                role: "AXButton".into(),
                subrole: Some("AXCloseButton".into()),
                label: Some("Close".into()),
                frame_image: Some((10, 20, 30, 40)),
                frame_global: Some((11.0, 21.0, 30.0, 40.0)),
                enabled: true,
                focused: false,
                ax_actionable: true,
            }],
            tree_text: "[0] AXButton \"Close\"".into(),
            digest: "abc123".into(),
            captured_at_ms: 1700000000000,
            screenshot: None,
            loop_warning: None,
        };
        let v = serde_json::to_value(&view).expect("serialize view");
        assert_eq!(v["digest"], "abc123");
        assert_eq!(v["elements"][0]["i"], 0);
        assert_eq!(v["elements"][0]["node_idx"], 17);
        let back: InteractiveView = serde_json::from_value(v).expect("deserialize view");
        assert_eq!(back, view);
    }

    #[test]
    fn click_index_target_serializes_with_kind_tag() {
        let by_idx = ClickIndexTarget::Index { i: 5 };
        let v = serde_json::to_value(&by_idx).expect("serialize");
        assert_eq!(v["kind"], "index");
        assert_eq!(v["i"], 5);
        let by_node = ClickIndexTarget::NodeIdx { idx: 9 };
        let v = serde_json::to_value(&by_node).expect("serialize");
        assert_eq!(v["kind"], "node_idx");
        assert_eq!(v["idx"], 9);
    }

    #[test]
    fn interactive_click_params_apply_defaults() {
        let parsed: InteractiveClickParams =
            serde_json::from_value(serde_json::json!({"i": 3})).expect("deserialize minimal click params");
        assert_eq!(parsed.i, 3);
        assert_eq!(parsed.click_count, 1);
        assert_eq!(parsed.mouse_button, "left");
        assert!(parsed.modifier_keys.is_empty());
        assert!(parsed.return_view);
    }

    #[test]
    fn visual_mark_params_apply_defaults() {
        let opts: VisualMarkViewOpts = serde_json::from_value(serde_json::json!({})).expect("deserialize minimal opts");
        assert_eq!(opts.max_points, None);
        assert_eq!(opts.region, None);
        assert!(opts.include_grid);

        let click: VisualClickParams =
            serde_json::from_value(serde_json::json!({"i": 5})).expect("deserialize minimal visual click params");
        assert_eq!(click.i, 5);
        assert_eq!(click.click_count, 1);
        assert_eq!(click.mouse_button, "left");
        assert!(click.return_view);
    }

    #[test]
    fn interactive_type_text_params_round_trip() {
        let params = InteractiveTypeTextParams {
            i: Some(7),
            text: "hello".into(),
            clear_first: true,
            press_enter_after: true,
            before_view_digest: Some("d".into()),
            wait_ms_after: Some(100),
            return_view: true,
        };
        let v = serde_json::to_value(&params).expect("serialize");
        let back: InteractiveTypeTextParams = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, params);
    }

    #[test]
    fn interactive_scroll_params_apply_defaults() {
        let parsed: InteractiveScrollParams =
            serde_json::from_value(serde_json::json!({"i": None::<u32>})).expect("deserialize minimal scroll params");
        assert_eq!(parsed.i, None);
        assert_eq!(parsed.dx, 0);
        assert_eq!(parsed.dy, 0);
        assert!(parsed.return_view);
    }

    #[test]
    fn app_wait_predicate_round_trips_each_variant() {
        for pred in [
            AppWaitPredicate::DigestChanged {
                prev_digest: "abc".to_string(),
            },
            AppWaitPredicate::TitleContains {
                needle: "Save".to_string(),
            },
            AppWaitPredicate::RoleEnabled {
                role: "AXButton".to_string(),
            },
            AppWaitPredicate::NodeEnabled { idx: 12 },
        ] {
            let v = serde_json::to_value(&pred).expect("serialize predicate");
            let back: AppWaitPredicate = serde_json::from_value(v).expect("deserialize predicate");
            assert_eq!(back, pred);
        }
    }
}
