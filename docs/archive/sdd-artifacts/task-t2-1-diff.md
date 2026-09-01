diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 5fa0e42..a88afeb 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -95,10 +95,11 @@ jobs:
         # generated_locale_contract.rs is gitignored; northhing-core fails E0583 without it
 
       - name: Check compilation
-        run: cargo check --workspace --exclude northhing-cli --exclude northhing
+        run: cargo check --workspace
 
-      - name: Run core Rust tests
-        run: cargo test --locked -p northhing-core
+      - name: Run workspace Rust tests
+        if: matrix.os == 'ubuntu-latest'
+        run: cargo test --locked --workspace
 
   # ── kernel-api dependency guard ─────────────────────────────────────
   kernel-api-clean:
diff --git a/docs/architecture/backend-roadmap.md b/docs/architecture/backend-roadmap.md
index daad12a..0a2a22e 100644
--- a/docs/architecture/backend-roadmap.md
+++ b/docs/architecture/backend-roadmap.md
@@ -39,7 +39,7 @@
 | **K3 kernel 下沉** | **闸门待重裁** | 原判定"符合降级条件"（编译目标已达成）；**热重载目标改变 ROI**——进程外 core 需要 facade 实现移出 assembly/core，K3 从"认知重构"变"物理拆分前置"。待用户正式裁定并回写北极星 |
 | K5 收尾 | 随 K3 缩放 | 不变量入 AGENTS.md + 编译对比报告 |
 
-北极星既定纪律（继续有效）：P2 面扩评审（N×1.2）、facade 禁 re-export 内部泛型/derive、`cargo tree -p northhing-kernel-api` 守卫入 CI（**尚未入 CI，见 T2-1**）。
+北极星既定纪律（继续有效）：P2 面扩评审（N×1.2）、facade 禁 re-export 内部泛型/derive、`cargo tree -p northhing-kernel-api` 守卫入 CI（**已在 CI，kernel-api-clean job**）。
 
 ### 1.2 P 线（插件化与热重载）——方向已裁，并入本文 T5
 
@@ -163,7 +163,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 
 | # | 内容 | 来源线 | 量 |
 |---|---|---|---|
-| T2-1 | **CI 补齐**：check 去 exclude、test 扩面、`cargo tree -p northhing-kernel-api` 守卫 job（北极星 §4 既有要求）、desktop check 强制门（P2-15 流程结转） | K+review | S |
+| T2-1 | **CI 补齐**：check 去 exclude、test 扩面、`cargo tree -p northhing-kernel-api` 守卫已在 CI（kernel-api-clean job）、desktop check 强制门（P2-15 流程结转） | K+review | S |
 | T2-2 | 死代码删除第一批（insights / tool-provider-groups / 空 session 目录 / webdriver / enigo+screenshots / **judge_gate 适配层**（assembly/core 1,690L；**协议层 1,473L 保留**转 TH-5 词汇，2026-08-17 G15 修正）≈6.5k 行）**+ remote 栈整删（TH-4：remote_connect 11.5k + mobile-web 4.7k + embedded relay 入口先摘后删；P1-4/P1-7/D-2 随之关闭）** **+ MiniApp 子系统整删（2026-08-17 拍板：内置四件套 + 宿主 host_routing/bridge/manager/契约 ≈6k 行；permission_policy 默认拒绝语义先提炼进 PCS 设计再删码；连带关闭 T1-1、T3-5）**+ relay-server + relay-core 整删（PEND-1 拍板 2026-08-17：≈4-5k 行；surfaces.md 同 commit 同步）** + plan-compliance-checker(894L) + harness(571L，或并入 test-support)**，合计 ≈35k 行 | review+论题 | M |
 | T2-9 | **功能冗余合并批次**（2026-08-17 冗余扫描）：第一批 S 级——deep_research 去重（255L×2，diff 仅 10 行注释→re-export）、ndjson_log 统一（4 个追加+轮转实现 ~1,320L）、now_unix_ms 统一（3 同名函数+25 内联）、原子写收口 json_store（顺修 P2-16 save_config 裸写；删 PersistenceService FILE_LOCKS）、初始化收口（server bootstrap 手抄 + CLI 样板×4 → init_agentic_system）；第二批 M 级——app.json↔GlobalConfig 镜像拆除（写穿 kernel API）、**事件管道收敛 A7**（BackendEvent 死管道并入 EventQueue 或删除）、**desktop NullDispatcher 空转路径移除**（agent-dispatch B2，回退直连直至 dispatcher 真接线）；延期 L 级——ExecCommand↔Bash 合并（Bash/PTY 为正）、双 ToolRegistry 迁移收尾、MCP core 包装层（3,641L）收口 | 冗余扫描 | 第一批 S / 第二批 M / 延期 L |
 | T2-10 | **连续性自检测试**：自动化"杀 core → 恢复 → diff 会话/记忆/身份"（T5"agent 不死"验收的轻量前置版，0.3 即可写，依赖 fake AI backend 提供确定性） | 论题 §3 度量 | S |
diff --git a/docs/status/tech-debt-ledger.md b/docs/status/tech-debt-ledger.md
index 3002a22..e9e406c 100644
--- a/docs/status/tech-debt-ledger.md
+++ b/docs/status/tech-debt-ledger.md
@@ -191,7 +191,7 @@
 - **Evidence**: Discovered 2026-08-05 while dispatching Task B3 of the backend follow-ups round; fixed by commit `b0bfe43` (keyring `v1` feature + 3 API/`Lazy` compile fixes + one test import path, zero behavior change, judge-verified line by line). New desktop baseline: `cargo test -p northhing --lib` = 118/118.
 - **Root cause (process)**: a security-sensitive change was accepted on a report whose verification section was incomplete, and the round handoff reused an older desktop test figure instead of a fresh measurement.
 - **Proposed fix**: gate it structurally — `cargo check -p northhing` must pass before any branch merges to main (recorded as housekeeping rule 6 in `AGENTS.md` / `AGENTS-CN.md`, 2026-08-06), and a round handoff must not carry forward a verification baseline it did not measure itself.
-- **Status**: code defect resolved (`b0bfe43`); process gate recorded 2026-08-06 (house rule 6). CI enforcement of the desktop check is still open.
+- **Status**: resolved (2026-08-17, T2-1: `cargo check --workspace` in CI includes `northhing` and `northhing-cli`; code defect resolved in `b0bfe43`; process gate recorded in housekeeping rule 6).
 
 ### P2-16: `ConfigManager::save_config` writes the whole config file non-atomically
 
