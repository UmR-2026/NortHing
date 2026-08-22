diff --git a/docs/status/tech-debt-ledger.md b/docs/status/tech-debt-ledger.md
index a2abaa5..1eecf8e 100644
--- a/docs/status/tech-debt-ledger.md
+++ b/docs/status/tech-debt-ledger.md
@@ -234,7 +234,7 @@
 - **Symptom**: MiniApp 子系统整删后，契约层保留了三处 serde/wire 残留：`core-types/src/surface.rs:52` `RuntimeArtifactKind::MiniApp`、`services-core/src/session/session_metadata.rs:27` `SessionRelationshipKind::Miniapp`、`services-core/src/session/lineage.rs:19` `"miniapp"` tag。当前代码中零构造、零生产者，但直接删除存在旧会话/工件数据反序列化兼容风险。
 - **Evidence**: T2-2 MiniApp recon Q7 (`.superpowers/sdd/task-t2-2-miniapp-recon.md`)；`rg` 实测全仓零业务构造。
 - **Proposed fix**: 2026-08-19 用户决策超时未拍板，默认保守路径悬置待决。后续若确认无旧数据迁移负担可整删变体，或在反序列化层增加 serde alias/fallback 后删除。
-- **Status**: active (suspended / pending user decision)
+- **Status**: `resolved` — 用户 2026-08-19 拍板删除，本任务执行，commits 见 git log T2-2p。
 
 ## Change Protocol
 
diff --git a/src/crates/contracts/core-types/src/surface.rs b/src/crates/contracts/core-types/src/surface.rs
index 33f2ac1..3216c9b 100644
--- a/src/crates/contracts/core-types/src/surface.rs
+++ b/src/crates/contracts/core-types/src/surface.rs
@@ -49,7 +49,6 @@ pub enum RuntimeArtifactKind {
     Preview,
     Usage,
     ReviewReport,
-    MiniApp,
     McpManifest,
 }
 
diff --git a/src/crates/services/services-core/src/session/lineage.rs b/src/crates/services/services-core/src/session/lineage.rs
index 7b119f3..977ca9a 100644
--- a/src/crates/services/services-core/src/session/lineage.rs
+++ b/src/crates/services/services-core/src/session/lineage.rs
@@ -16,7 +16,7 @@ const LINEAGE_CUSTOM_METADATA_KEYS: &[&str] = &[
     "subagentType",
 ];
 
-const BRANCH_EXCLUDED_TAGS: &[&str] = &["btw", "review", "deep_review", "miniapp", "subagent"];
+const BRANCH_EXCLUDED_TAGS: &[&str] = &["btw", "review", "deep_review", "subagent"];
 
 #[derive(Debug, Clone, PartialEq, Eq)]
 struct SubagentRelationshipFacts {
diff --git a/src/crates/services/services-core/src/session/session_metadata.rs b/src/crates/services/services-core/src/session/session_metadata.rs
index ebe2800..2062b2c 100644
--- a/src/crates/services/services-core/src/session/session_metadata.rs
+++ b/src/crates/services/services-core/src/session/session_metadata.rs
@@ -24,7 +24,6 @@ pub enum SessionRelationshipKind {
     Btw,
     Review,
     DeepReview,
-    Miniapp,
     Subagent,
 }
 
