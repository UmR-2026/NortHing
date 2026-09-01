// 此文件因 A 类「未初始化断言」单测独占进程而独立成文件
// 不要向本文件添加任何会触发 init_core() / init_*() / 全局单例初始化的测试
// 违反即回归

use northhing::ui_dioxus::api::ensure_room_session;

#[tokio::test]
async fn test_ensure_room_session_fails_cleanly_when_uninitialized() {
    let res = ensure_room_session().await;
    assert!(res.is_err());
}
