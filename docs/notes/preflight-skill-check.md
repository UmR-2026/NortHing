# Skill ç³»ç»ä½¿ç¨è¯´æ (northhing-v3)

> æ¥æ northhing-v3 é¡¹ç®ç?agent **å¿è¯»**ãè¯´ææ¬é¡¹ç®ç¨åªä¸?skill ä½ç³»ãä¸ºä»ä¹ãä»¥åæ¥å¸¸æä¹ç¨ã?
## TL;DR

- **å¨å± meta-skill**ï¼`preflight-skill-check`ï¼ä½äº?`C:\Users\UmR\.agents\skills\preflight-skill-check\`ï¼?- **æ¬é¡¹ç®ç¶æ?*ï¼`docs/PROJECT_STATE.md` æ«å°¾ "ð§ Skill ç³»ç»ç°ç¶ (2026-06-18)" æ®µè½
- **å®åæ¥æº**ï¼ç¨æ·æ¾å¼è¦æ±éæ?skill-listï¼å¹¶ç¡®ç«"å¤æ­éæ±æ¯å¦å¯ä»¥åè°ç¨ skill æ¥è¿è¡ä»»å?çå¤å®å®å?- **åå»ºæ¥æ**ï¼?026-06-18

## ä¸ºä»ä¹æè¿ä¸ª skill

ä¹åç¨æ·å¤æ¬¡åé¦ï¼?agent å¿½ç¥äºæè¯´ç skill"ã?ä¸ºä»ä¹æ²¡è°?brainstorming å°±å¼å§åä»£ç äº?ã?
æ ¹å ï¼?*ZCode ç?`Skill` å·¥å·åªæ¥å?system-reminder åè¡¨éç"æè½½" skill**ãæ¬ä¼è¯åªæè½?4 ä¸ªï¼`docx`, `pdf`, `skill-creator`, `using-coze-cli`ï¼ï¼ä½ç£çä¸è¿æ 18 ä¸ªå·²å®è£ç?skillï¼superpowers å¨å¥ 16 ä¸?+ å¹³å°ç¹å® 2 ä¸?+ æ¢å¤ä¼è¯ 1 ä¸ªï¼ãè¿äº?skill ä»ç¶éç¨ï¼ä½æ¨¡åç»å¸¸å ä¸ºçä¸å°è§¦åææ¬èè·³è¿ã?
`preflight-skill-check` è§£å³ä¸¤ä»¶äºï¼
1. **å¼ºå¶é¢æ£**ï¼æ¯ä¸?turn å¼å§åæ«å·²æè½½ + å·²å®è£?skillï¼å¹éè§¦åä¿¡å?2. **æªæè½?fallback**ï¼å½éè¦ç skill æªæè½½æ¶ï¼æç¡®åè¯ç¨æ·æ Read SKILL.md åæå¨éµå¾?
## æ¥æ northhing æ¶çå¼ºå¶æµç¨

æ¯æ¬¡æ¥å° northhing ç¸å³ä»»å¡ï¼ä¸ç®¡æ¯ä¿?bugãå  featureãéæãè°è¯ï¼ï¼?
```
1. è¯?docs/PROJECT_STATE.mdï¼åç?"ä¸å¥è¯ç¶æ? + "Skill ç³»ç»ç°ç¶"ï¼?2. è¯»æ¬æä»¶ï¼ç¡®è®?skill è°ç¨çº¦å®ï¼?3. æ?preflight-skill-check ç?4 æ­¥èµ°
4. æ¯æ¬¡è°?Skill å·¥å·åè¾åºä¸è¡?"Using <skill> to <purpose>"
```

## å¸åè°ç¨é¾ï¼northhing åºæ¯ï¼?
| åºæ¯ | å»ºè®®é¡ºåº |
|---|---|
| æ°åè½ï¼å¦?Phase R1 shell sandboxï¼?| `brainstorming` â?`writing-plans` â?`test-driven-development` â?impl â?`verification-before-completion` |
| ä¿?bugï¼å¦æä¸ª tool å¡ä½ï¼?| `systematic-debugging` â?`test-driven-development` â?`verification-before-completion` |
| åºç¨ CODE_REVIEW ä¿®å¤ | ï¼ç´æ¥åï¼â `verification-before-completion` â?commit |
| å?æ?skill | `skill-creator`ï¼å« test prompt éªè¯ï¼?|
| å®æåæ¯ | `verification-before-completion` â?`finishing-a-development-branch` |
| è¯?PDF / DOCX | `pdf` / `docx`ï¼æè½½ï¼ç´æ¥è°ï¼ |

## è·?northhing ç°æçº¦å®çä¸è´æ?
- **Commit ä¹ æ¯**ï¼v3-restructure åæ¯ä½¿ç¨ const-flag pattern + 1 commit per logical change + æ´æ° PROJECT_STATE.mdãæ¬ skill åå»ºæ¶å 1 ä¸?docs åæ´ï¼PROJECT_STATE.md "Skill ç³»ç»ç°ç¶" æ®µè½ï¼ï¼ç¬ç« commitã?- **è·¯å¾å®æ´**ï¼ææè·¯å¾é½ç¨ç»å¯?Windows è·¯å¾ï¼ç¬¦å?v3 é¡¹ç®åå¥½ã?- **ä¸æèªå¨ v3 ä»£ç **ï¼æ¬ skill åå»º**ä»æ¹å?PROJECT_STATE.md**ï¼ä¸å¨ä»»ä½?Rust/TS ä»£ç ã?- **Pitfall è®°å½**ï¼å¨ PROJECT_STATE.md çç¸å³æ®µè½ä¸­æå°äºå¸¸è§è¯¯åºï¼ç³»ç»æè½½â å¨é¨å¯ç¨ï¼ã?
## æªæè½?skill çå¤çæ­¥éª¤ï¼å®éæä½ï¼?
åè®¾ææ¬¡éè¦?`brainstorming` ä½å®æ²¡æè½½ï¼

```text
1. Read: C:\Users\UmR\.zcode\cli\plugins\cache\zcode-plugins-official\superpowers\5.1.0\skills\brainstorming\SKILL.md
2. å¨ååºéæç¡®å®£å¸ï¼?   "Using brainstorming (manually loaded from <path>) to clarify scope before writing code."
3. æ?SKILL.md ç?9-step checklist è¡äº
4. ä¸è¦"å­å°è±?èµ?brainstorming æµç¨ââææ¡£å¯è½å·²æ¼è¿
```

## æµè¯éªè¯ï¼æå¨ï¼

è®¾è®¡ææ¡£éç»äº?3 æ?test promptï¼è¯¦è§?preflight-skill-check çè®¾è®¡è®¨è®ºï¼ï¼?
- **T1**ï¼?å¸®ææ?northhing ç?shell æ²ç®±åäº" â?åºè§¦å?brainstorming
- **T2**ï¼?è¯»ä¸ä¸?docs/spec.pdf ç¶åæ»ç»è¦ç¹" â?åºè§¦å?pdf
- **T3**ï¼?Cargo.lock éæä¸?typo æ¹ä¸ä¸? â?ä¸åºè§¦åä»»ä½ skillï¼ç´æ?Edit

ä¸æ¬¡æ°ä¼è¯å¼å§æ¶ï¼è·è¿?3 æ¡éªè¯?skill è§¦åè¡ä¸ºæ¯å¦åçã?
## æ¼è¿è§å

- `preflight-skill-check` æ¬èº«æ¹äº â?æ´æ° `C:\Users\UmR\.agents\skills\preflight-skill-check\CHANGELOG.md`
- é¡¹ç®ä¾?skill çº¦å®æ¹äº â?æ´æ°æ¬æä»?+ `PROJECT_STATE.md` ç?"Skill ç³»ç»ç°ç¶" æ®µè½
- ä»»ä½åæ´åç»ç¨æ·æ¹åï¼æ brainstorming HARD-GATEï¼?
## ç¸å³é¾æ¥

- ä¸?skillï¼`C:\Users\UmR\.agents\skills\preflight-skill-check\SKILL.md`
- Skill è¯¦è¡¨ï¼`C:\Users\UmR\.agents\skills\preflight-skill-check\references\skill-catalog.md`
- é¡¹ç®ç¶æï¼`docs/PROJECT_STATE.md`ï¼?ð§ Skill ç³»ç»ç°ç¶ (2026-06-18)" æ®µï¼
- åæ´æ¥å¿ï¼`C:\Users\UmR\.agents\skills\preflight-skill-check\CHANGELOG.md`
- using-superpowers åçï¼`C:\Users\UmR\.zcode\cli\plugins\cache\zcode-plugins-official\superpowers\5.1.0\skills\using-superpowers\SKILL.md`
