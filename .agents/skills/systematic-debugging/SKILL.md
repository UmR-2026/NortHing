---
name: systematic-debugging
description: "Use when debugging non-trivial issues in the northhing Rust codebase. Enforces root-cause-first methodology: reproduce â?localize â?hypothesize â?fix â?guard. Activates when cargo build/test fails unexpectedly, when const flag behavior is wrong, or when a refactor introduces subtle bugs. Trigger this BEFORE attempting any fix."
---

# Systematic Debugging (northhing v3 ééç?

> åå¹¶è?obra/superpowers systematic-debugging, mattpocock/skills diagnosing-bugs, addyosmani/agent-skills debugging-and-error-recovery

## When to trigger

- `cargo build` æ?`cargo test` åºç°éé¢æå¤±è´?- const flag å¼å¥åè¡ä¸ºå¼å¸¸ï¼æä¸ª flag ç»åä¸?bugï¼?- éæååå½æµè¯å¤±è´?- è¿è¡æ?panic æé»è¾éè¯¯
- **3 æ¬¡çæµå¼ä¿®å¤å¤±è´¥åå¿é¡»è§¦å?*ï¼å¼ºå¶ä» Phase 1 éæ°å¼å§ï¼

## The Iron Rule

**æ²¡æå¯å¤ç°ççº¢ç¯å½ä»¤ï¼ä¸è®¸è¿å¥ä¿®å¤é¶æ®µã?*

---

## Phase 1: æå»ºåé¦å¾ªç¯ (Build the Feedback Loop)

ç®æ ï¼æ¾å°ä¸æ?**å¿«éãç¡®å®æ§ãçº¢è?* çå½ä»¤æ¥æ´é²é®é¢ã?
### Rust å·¥å·é¾éé¡¹ï¼æéåº¦æåºï¼?
| å½ä»¤ | éç¨åºæ¯ | éåº¦ |
|------|---------|------|
| `cargo check -p <crate>` | ç¼è¯éè¯¯ | ~5s |
| `cargo test -p <crate> --lib <test_name>` | ååæµè¯å¤±è´¥ | ~10s |
| `cargo test -p <crate> --lib -- --nocapture` | éè¦ç println! è¾åº | ~10s |
| `cargo build -p <crate>` | å®æ´ç¼è¯ï¼å« codegenï¼?| ~30s |
| `cargo test --workspace` | å¨éåå½ | ~2min |
| `cargo clippy -p <crate>` | lint çº§å«é®é¢ | ~15s |

**æ¾å°é£æ¡è½ç¨³å®å¤ç°é®é¢çæå¿«å½ä»¤ã?* åä¸æ¥ï¼

```
RED_COMMAND: cargo test -p northhing-core --lib test_prompt_loading
```

### const flag ç¹æ®åºæ¯

å¦æ bug åªå¨ç¹å® flag ç»åä¸åºç°ï¼
```bash
# éç¦»æµè¯ï¼éä¸ªå³é­ flag
const USE_X: bool = false;  // éä¸ªææ¥
const USE_Y: bool = true;
const USE_Z: bool = true;
```

**å¦ææ æ³å¤ç° â?ä¸è¦çæµä¿®å¤ï¼åå»ºåé¦å¾ªç¯ã?*

---

## Phase 2: å¤ç° + æå°å (Reproduce & Minimize)

1. ç?RED_COMMAND ç¡®è®¤å¯å¤ç?2. **æå°å**ï¼éæ­¥ç§»é¤æ å³ä»£ç ï¼ç´å°åªå©è§¦å?bug çæå°è·¯å¾?3. å¦ææ?const flag ç¸å³ï¼éä¸ªå³é­å¶ä» flagï¼æ¾å°è§¦åç»å?
### Git Bisectionï¼v3 ç¹åï¼?
å½ä¸ç¡®å®åªä¸ª commit/flag å¼å¥äºé®é¢ï¼
```bash
cd E:/agent-project/northhing-v3
git bisect start
git bisect bad              # å½åç¶ææ¯ bad
git bisect good <commit>    # ä¸ä¸ä¸ªå·²ç?good ç?commit
# æ¯ä¸ª step: cargo test -p <crate> --lib
```

---

## Phase 3: åè®¾ä¸éªè¯?(Hypothesize & Test)

### è§å

- **æ¯æ¬¡åªéªè¯ä¸ä¸ªåè®?*
- åè®¾å¿é¡»å¯è¯ä¼?- åä¸é¢æç»æï¼åçå®éç»æ?
### æ¨¡æ¿

```
HYPOTHESIS #1: <å¯¹æ ¹å ççæµ>
EVIDENCE: <æ¯æè¿ä¸ªåè®¾çä»£ç ?æ¥å¿/ç¼è¯è¾åº>
TEST: <å¦ä½éªè¯ â?ä¿®æ¹ä»ä¹ï¼ææä»ä¹ç»æ?
EXPECTED: <å¦æåè®¾æ­£ç¡®ï¼åºè¯¥çå°ä»ä¹?
ACTUAL: <å®éç»æ>
VERDICT: <confirmed / refuted / inconclusive>
```

### Rust å¸¸è§æ ¹å æ¨¡å¼

| çç¶ | å¯è½æ ¹å  | éªè¯æ¹å¼ |
|------|---------|---------|
| ç¼è¯éè¿ä½è¡ä¸ºéè¯?| const flag ç?if/else åæ¯é»è¾åäº | æ£æ?`if FLAG {} else {}` ç?true/false å¯¹åºåªæ¡è·¯å¾ |
| trait bound éè¯¯ | éææ¹åäº?trait impl çå¯è§æ?| `cargo doc -p <crate> --open` ç?trait å®ç° |
| panic on short input | slice æä½æ²¡æè¾¹çæ£æ?| æ£æ?`[..N]` æ¯å¦æé¿åº¦ä¿æ?|
| feature unification é®é¢ | workspace æåç?feature å²çª | `cargo tree -e features -p <crate>` |
| æµè¯éè¿ä½çäº?panic | æµè¯åªè¦çäº happy path | æ£æ?`#[should_panic]` å?edge case è¦ç |

---

## Phase 4: å®ç°ä¿®å¤ (Fix)

1. **ååå¤±è´¥æµè¯**ï¼å¼ç?`test-driven-development` skillï¼?2. ç¡®è®¤æµè¯å¤±è´¥ï¼çº¢ç¯ï¼
3. åæå°ä¿®å¤?4. ç¡®è®¤æµè¯éè¿ï¼ç»¿ç¯ï¼
5. è·å¨éåå½ï¼`cargo test -p <crate> --lib`
6. è·?clippyï¼`cargo clippy -p <crate> -- -D warnings`

### const flag ä¿®å¤æ£æ¥æ¸å?
- [ ] ä¿®å¤ä¸æ¹å?flag çé»è®¤å¼ï¼é¤é bug å°±æ¯é»è®¤å¼éè¯¯ï¼
- [ ] if/else ä¸¤ä¸ªåæ¯é½æµè¯å°
- [ ] ä¿®å¤æ²¡æå¼å¥æ°ç panic è·¯å¾
- [ ] ç¸å³ç?regression test éè¿

---

## Phase 5: é²æ­¢å¤å (Guard)

1. **ä¿çè§¦å bug ç?regression test**ï¼å·²ç»åå¥½äºï¼?2. å¦ææ ¹å æ?"æä¸ªæ¨¡å¼å®¹æåé"ï¼èèå?lint æ?clippy è§å
3. æ´æ° `CODE_REVIEW.md` å¦æè¿æ¯ä¸ä¸ªå¼å¾è®°å½çé·é?
---

## ä¸æ¬¡å¤±è´¥è§å

å¦æä½ å°è¯ä¿®å¤?**3 æ¬?* ä»ç¶æ²¡æè§£å³é®é¢ï¼?
**åã?* ä¸è¦ç»§ç»­çæµã?
1. åå° Phase 1ï¼ç¡®è®¤ä½ ç?RED_COMMAND ççæ¯çº¢è²ç
2. åå° Phase 3ï¼è´¨çä½ æåºæ¬çåè®?3. å¦æ 5 ä¸ªåè®¾é½è¢«æ¨ç¿»ï¼èèï¼?   - æ¶æå±é¢çé®é¢ï¼const flag æ¨¡å¼æ¬èº«æ¯å¦éåè¿ä¸ªåºæ¯ï¼ï¼
   - ç?`git stash` åå°å·²ç¥ good ç¶æï¼éæ°å¼å§?   - è¯·å¶ä»?agent/äººå®¡æ¥ï¼ç?`code-review` skillï¼?
---

## ä¸å¶ä»?skill çå³ç³?
- **test-driven-development**: Phase 4 ç?"åå¤±è´¥æµè¯? ç´æ¥ä½¿ç¨
- **verification-before-completion**: Phase 5 çæç»éªè¯?- **code-review**: å¦æ bug åæ äºæ¶æé®é¢ï¼è§¦åå®¡æ¥
- **northhing-v3-workflow**: bug ä¿®å¤ä¹éµå¾?const flag æ¨¡å¼ï¼å¦ææ¯æ?flag ç?bugï¼?