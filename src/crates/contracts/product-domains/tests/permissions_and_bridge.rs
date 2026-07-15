#![cfg(feature = "miniapp")]

//! Permissions And Bridge tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn miniapp_csp_content_preserves_net_allow_contract() {
    let permissions = MiniAppPermissions {
        net: Some(NetPermissions {
            allow: Some(vec!["api.example.com".to_string()]),
        }),
        ..MiniAppPermissions::default()
    };

    let csp = build_csp_content(&permissions);

    assert_eq!(
        csp,
        "default-src 'none'; script-src 'self' 'unsafe-inline' 'unsafe-eval' https:; style-src 'self' 'unsafe-inline' https:; connect-src 'self' 'self' https://esm.sh api.example.com; img-src 'self' data: https:; font-src 'self' https:; object-src 'none'; base-uri 'self';"
    );
}


#[test]
fn miniapp_permissions_support_host_notifications_without_domain_specific_fields() {
    let permissions: MiniAppPermissions = serde_json::from_value(serde_json::json!({
        "notifications": { "system": true },
        "net": { "allow": ["*"] }
    }))
    .unwrap();

    assert_eq!(
        permissions.notifications,
        Some(NotificationPermissions { system: true })
    );
    assert_eq!(permissions.net.unwrap().allow.unwrap(), vec!["*"]);
}


#[test]
fn miniapp_bridge_exposes_host_notification_namespace() {
    let bridge = build_bridge_script("app-1", "/tmp/app", "/tmp/workspace", "dark", "win32");

    assert!(bridge.contains("notifications:"));
    assert!(bridge.contains("notifications.system"));
    assert!(bridge.contains("system:"));
    assert!(bridge.contains("system.openExternal"));
}


#[test]
fn miniapp_bridge_exposes_deck_render_page_namespace() {
    let bridge = build_bridge_script("app-1", "/tmp/app", "/tmp/workspace", "dark", "win32");

    assert!(bridge.contains("deck:"));
    assert!(bridge.contains("deck.renderPage"));
}


#[test]
fn miniapp_permission_policy_preserves_scope_resolution() {
    let permissions = MiniAppPermissions {
        fs: Some(FsPermissions {
            read: Some(vec!["{appdata}".to_string(), "{workspace}".to_string()]),
            write: Some(vec!["{user-selected}".to_string()]),
        }),
        ..MiniAppPermissions::default()
    };

    let policy = resolve_policy(
        &permissions,
        "app_1",
        Path::new("/tmp/app-data"),
        Some(Path::new("/tmp/workspace")),
        &[PathBuf::from("/tmp/granted")],
    );

    assert_eq!(policy["fs"]["read"][0], "/tmp/app-data");
    assert_eq!(policy["fs"]["read"][1], "/tmp/workspace");
    assert_eq!(policy["fs"]["read"][2], "/tmp/granted");
    assert_eq!(policy["fs"]["write"][0], "/tmp/granted");
}


