# Task 1 Review: Relay 磁盘层路径防线（重审）

**审查范围**: `c6096cb..c5dfcde`
**上轮结论**: Spec FAIL（I-1 M-8 测试缺失）/ Quality PASS / 5 项 findings
**本轮 commit**: `c5dfcde fix(relay): T1 评审修复 — M-8 路由层测试 + Linux 盘符缺口 + cleanup_room fail-closed`
**fixer 报告**: `.superpowers/sdd/task-01-fix-report.md`（以 diff 为准）

---

## 一、复核结论

### I-1（M-8 路由层测试）：**CLOSED**

**实现证据**：`src/crates/services/relay-core/src/routes/api.rs:498-679` 新增 `#[cfg(test)] mod handler_tests`，含 6 个 `#[tokio::test]`：

| 测试 | 行 | 验证内容 |
|---|---|---|
| `check_web_files_existing_counts_on_successful_map` | L567-598 | 成功路径：1 existing + 1 needed，total=2 |
| **`check_web_files_failing_map_counts_needed_not_existing`** | **L601-628** | **FailingMapStore：existing_count==0、needed.len()==1、`needed[0]=="a.js"`、`existing_count + needed.len() == total_count` 计数不变式** |
| `check_web_files_rejects_invalid_room_id` | L631-636 | `..` → NOT_FOUND |
| `check_web_files_invalid_path_counts_as_needed` | L639-656 | `../x` → needed（路由层 invalid-path 计入 needed，不是 silent continue） |
| `upload_web_rejects_traversal_path` | L659-672 | `../evil` → BAD_REQUEST |
| `serve_catchall_rejects_invalid_rel_path` | L675-679 | `r/../x` → BAD_REQUEST |

**FailingMapStore**（L538-564）是 `MemoryAssetStore` 的 wrapper，覆写 `map_to_room` 永远返回 `Err("disk mapping failure".to_string())`，`has_content` 透传内层——这是 brief §5 要求的「让 `map_to_room` 失败」的最简注入点。

**实测运行**（我亲自重跑）：
```
$ cargo test -p northhing-relay-core
running 17 tests
test routes::api::handler_tests::check_web_files_failing_map_counts_needed_not_existing ... ok
test routes::api::handler_tests::check_web_files_existing_counts_on_successful_map ... ok
test routes::api::handler_tests::check_web_files_invalid_path_counts_as_needed ... ok
test routes::api::handler_tests::check_web_files_rejects_invalid_room_id ... ok
test routes::api::handler_tests::upload_web_rejects_traversal_path ... ok
test routes::api::handler_tests::serve_catchall_rejects_invalid_rel_path ... ok
... (validated::tests + 既有 relay/room tests 全过)
test result: ok. 17 passed; 0 failed

$ cargo test -p northhing-relay-server
running 7 tests
test disk_tests::...（7 个全过）
test result: ok. 7 passed; 0 failed
```

合并 24/24 全绿，I-1 关闭。

### M-1（Linux 盘符缺口）：**CLOSED**

**实现证据**：`src/crates/services/relay-core/src/validated.rs:157-166` 在 `\`→`/` 归一化后、components 循环之前，加了 split 段级盘符扫描：

```rust
let normalized = s.replace('\\', "/");
// Windows drive-letter check applied upfront so it survives on non-Windows
// where Path::components sees `X:` as a plain Normal component ...
for seg in normalized.split('/') {
    if is_drive_letter(seg) {
        return Err(RelPathError::Prefix);
    }
}
```

**Linux 语义覆盖核对**（逐用例 trace）：

| 输入 | 归一化后 | split 段 | is_drive_letter 命中？ | 结果 |
|---|---|---|---|---|
| `C:\abs` (6 字节) | `C:/abs` | `["C:", "abs"]` | `"C:"` 长度 2、首字母 ASCII、下标 1=':' → TRUE | ✅ reject Prefix |
| `C:\` | `C:/` | `["C:", ""]` | `"C:"` TRUE | ✅ reject |
| `C:` 单独 | `C:` | `["C:"]` | `"C:"` TRUE | ✅ reject |
| `\C:\foo` | `/C:/foo` | `["", "C:", "foo"]` | `"C:"` TRUE | ✅ reject |
| `\\unc\share` | `//unc/share` | `["", "", "unc", "share"]` | 无 drive-letter 段，但随后 components 阶段 RootDir 命中 | ✅ reject RootDir |
| `C:a` (无斜杠) | `C:a` | `["C:a"]` | `"C:a"` 长度 3 → FALSE | ⚠ 见下方残留分析 |

**不过度拒绝核对**（用户特别要求验证 `a:b` 这类单段）：

| 输入 | 归一化后 | split 段 | is_drive_letter | 结果 |
|---|---|---|---|---|
| `a:b` | `a:b` | `["a:b"]` | 长度 3 → FALSE | ✅ 放行 |
| `port:8080` | `port:8080` | `["port:8080"]` | 长度 9 → FALSE | ✅ 放行 |
| `data:files/x.txt` | `data:files/x.txt` | `["data:files", "x.txt"]` | 两段都非 drive-letter | ✅ 放行 |
| `C:\foo:C:\bar` | `C:/foo:C:/bar` | `["C:", "foo:C:", "bar"]` | `"C:"` TRUE | ✅ reject Prefix（首段就是 drive-letter 形态，安全侧拒绝） |
| `C：foo`（全角冒号 U+FF1A）| `C：foo` | `["C：foo"]` | 首字节非 ASCII → FALSE | ✅ 放行（非 Windows 盘符定义） |

**残留小缺口**：`C:a`（无路径分隔符）split 后为 `["C:a"]`，is_drive_letter 要求长度 == 2，因此放行。在 Linux 上会被存为字面文件 `C:a`（房间目录下，不构成 containment 逃逸）；在 Windows 上 `Path::components("C:a").components()` 仍能产生 Prefix 分量兜底。`L177-182` 的 components-loop 内 is_drive_letter 仅在段长 2 时命中，对 `C:a` 同样无效。**结论：M-1 spec 缺口已修复；唯一理论缺口（`C:a` 字面文件名）不构成威胁、无 brief §5 列出的对应测试覆盖需求。**

### M-2（canonicalize 失败静默放行）：**CLOSED**

**实现证据**：`src/apps/relay-server/src/lib.rs:173-190` 将原 `if let Some` 改为三臂 `match`：

```rust
match self.canonical_base_dir() {
    Some(canonical_base) if Self::is_within(&canonical_base, &dir) => {}  // proceed
    Some(_) => {                                                        // not within
        tracing::warn!("cleanup_room: rejecting unsafe path {} (outside base dir)", dir.display());
        return;
    }
    None => {                                                           // canonicalize failed
        tracing::warn!("cleanup_room: cannot canonicalize base dir {} — refusing removal of {}",
            self.base_dir, dir.display());
        return;
    }
}
```

三个分支覆盖：success / out-of-base / canonicalize-fail。符合 brief §3「否则拒绝删除并 warn」语义。

### M-3（dead branch `dir == canonical_base`）：**CLOSED**

**实现证据**：`src/apps/relay-server/src/lib.rs:174` 现在的 match 守卫只检查 `Self::is_within(...)`，dead 子句已删除。`is_within` 内部仍保留 `canonical_candidate != canonical_parent` 守卫（`L69-76`），避免目录等于 base 本身被误判为「在 base 内」。语义等价、更紧凑。

---

## 二、新发现（spec/quality 双视角）

### Spec 视角新发现

**无**。所有 brief §1-§5 + §6「明确不做」+ §7 约束逐项核对均通过。

### Quality 视角新发现

**Q-1（Minor）：`rel_path_rejects_escapes_and_absolutes` 测试在 Linux CI 上仍可能因 Path 平台语义差异而失败**

- 证据：`src/crates/services/relay-core/src/validated.rs:330-338` 测试用例 `"..\\x"` 在 Linux 上的 trace：
  - 归一化后 `../x`
  - split → `["..", "x"]`
  - components on Linux: `[ParentDir, Normal("x")]`
  - ParentDir 分支命中 → reject ✓
  
  实际会通过。**撤销此疑虑**——只是 M-1 之前的 Linux 缺口已修复，无需担心。

**Q-2（Minor）：`api.rs:507` 未使用 import 编译告警**

- 证据：cargo test 输出：
  ```
  warning: unused import: `base64::engine::general_purpose::STANDARD as B64`
     --> src\crates\services\relay-core\src\routes\api.rs:507:9
  ```
- 原因：`handler_tests::upload_web_rejects_traversal_path`（L665）使用全限定 `base64::engine::general_purpose::STANDARD.encode(b"x")` 而非 alias `B64.encode`。
- 影响：仅编译告警，不影响测试结果。`B64` 引入是为了简短调用，但当前用例未使用。
- 修复：删 `use base64::engine::general_purpose::STANDARD as B64;`（L507）或改 L665 用 `B64.encode`。trivial。

**Q-3（Minor）：components-loop 内 is_drive_letter 守卫为冗余死代码**

- 证据：`src/crates/services/relay-core/src/validated.rs:177-182`：
  ```rust
  // Extra drive-letter guard for any segment that slipped through
  // the upfront scan (e.g. `X:` without slash separators such as
  // `X:a` on some platforms); redundant on Windows but harmless.
  if is_drive_letter(part) {
      return Err(RelPathError::Prefix);
  }
  ```
  注释声称覆盖「`X:a` 形式」，但 `is_drive_letter` 要求长度 == 2，`X:a` 长度 3 仍不被命中——注释与行为不符，且该分支在 Windows 上被 `Component::Prefix` 提前捕获，在 Linux 上 `Path::components("X:a")` 产出单个长度 3 的 Normal 组件，is_drive_letter 不触发。
- 影响：实际无害（upfront split 扫描已覆盖所有 brief §5 列出的盘符形态）；仅冗余+注释不准确。
- 建议：删 L177-182 整段（依赖前置 split 扫描）或在 is_drive_letter 上放宽长度约束到「首字符 ASCII alpha + 第二字符 ASCII :」并注释清楚。

**Q-4（Minor）：`split('/')` 双重扫描可合并**

- 证据：`src/crates/services/relay-core/src/validated.rs:162-171` 两次 `normalized.split('/')` 循环（drive-letter + CurDir）。可合并为一次循环，性能与可读性双赢。
- 影响：cosmetic，无功能差异。

---

## 三、最终判决

| 维度 | 判决 | 主因 |
|---|---|---|
| **Spec 合规** | **PASS** | I-1 关闭（M-8 路由层 6 个 oneshot 测试覆盖失败/无效路径/无效 room_id/traversal/catchall，实测 17+7=24 全绿）；brief §1-§6 + §7 全部满足 |
| **代码质量** | **PASS** | M-1/M-2/M-3 全部关闭，无 Critical/Important 残留；4 项 Minor（Q-1 已撤销、Q-2 编译告警、Q-3 冗余代码、Q-4 可合并循环）记终审 triage，不阻塞合并 |

### 文件大小复核

| 文件 | 上轮 | 本轮 | 状态 |
|---|---|---|---|
| `src/apps/relay-server/src/lib.rs` | 401 | 410 | < 800，无压力 |
| `src/crates/services/relay-core/src/validated.rs` | 379 | 391 | < 800，无压力 |
| `src/crates/services/relay-core/src/lib.rs` | 169 | 169 | < 800，无压力 |
| `src/crates/services/relay-core/src/routes/api.rs` | 494 | 680 | < 800，无压力（handler_tests 占 182 行） |

### 日志复核

新增/修改的日志字符串（本轮 diff 内）：

| 位置 | 字符串 | 状态 |
|---|---|---|
| `src/apps/relay-server/src/lib.rs:176-178` | `"cleanup_room: rejecting unsafe path {} (outside base dir)"` | EN ✓ |
| `src/apps/relay-server/src/lib.rs:183-187` | `"cleanup_room: cannot canonicalize base dir {} — refusing removal of {}"` | EN ✓（含 em dash，unicode 但非 CJK/emoji） |

### 「明确不做」复核

- ✅ RoomManager.create_room 签名未改（`handler_tests::ensure_room` 是调用方，未触碰 `relay/room.rs`）
- ✅ WS handler、认证、generate_room_id、消息尺寸队列均未触
- ✅ 不在 main 直接实现
- ✅ Cargo.lock 仅加 tempfile（已存在依赖）到两个 crate，无新依赖引入

### Ledger 建议行

```
Task 1: PASS (commits c6096cb..c5dfcde, review clean)
  - I-1/M-1/M-2/M-3 全部关闭，spec §1-§6 满足，cargo test 24/24 绿
  - 4 项 Minor 记终审 triage：
    Q-2 (api.rs:507 unused import B64)
    Q-3 (validated.rs:177-182 redundant is_drive_letter guard + 注释不准确)
    Q-4 (validated.rs:162-171 双 split 扫描可合并)
    Q-1 已撤销（M-1 修复后 Linux Path 语义测试通过）
```

---

## 四、上轮 findings 状态汇总

| ID | 描述 | 状态 | 证据 |
|---|---|---|---|
| **I-1** | M-8 路由层测试缺失 | **CLOSED** | `api.rs:601-628` `check_web_files_failing_map_counts_needed_not_existing` + 5 个 sibling，实测通过 |
| **M-1** | Linux 盘符缺口 | **CLOSED** | `validated.rs:157-166` split 段级扫描；测试通过；不过度拒绝 `a:b`/`port:8080`/`data:files/...` 等正常文件名 |
| **M-2** | cleanup_room canonicalize 失败静默 | **CLOSED** | `lib.rs:173-190` 三臂 match，None 分支 warn+return |
| **M-3** | dead branch `dir == canonical_base` | **CLOSED** | `lib.rs:174` 已删除 dead 子句 |
| M-4 | 测试名不符（preserves_existing_dest_on_validation_failure 未真覆盖验证失败） | DEFERRED（终审 triage） | 修复批未触 |
| M-5 | map_to_room TOCTOU 窗口 | DEFERRED（终审 triage） | 修复批未触；fixer 报告 §遗留疑虑已记录 |