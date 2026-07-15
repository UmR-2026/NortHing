**ä¸­æ** | [English](AGENTS.md)

# Product Domains Agent æå

éç¨èå´ï¼`src/crates/contracts/product-domains`ã?

`northhing-product-domains` æ¿è½½å¯è±ç¦»å®æ?core runtime ç¼è¯çå¹³å°æ å³äº§åé¢åå¥çº¦ãè¿éåºèç¦çº¯ç¶æãDTOãç­ç¥åçª?
portsï¼å·ä½?runtime è¡ä¸ºä¸å±äºæ¬ crateã?

## æ¤æ 

- ä¸è¦è®?`northhing-product-domains` ä¾èµ `northhing-core`ã?
- ä¿æ default feature è½»éãé»è®¤æå»ºä¸å¾å¼å?runtimeãserviceãdesktopãnetworkãprocessãAI æ?tool-runtime ä¾èµã?
- æ?crate å¯ä»¥æ¿è½½çº?DTOãæä¸¾ãåºååå¥çº¦ãæç´¢è®¡åãå½ä»¤éæ©å³ç­ãstorage-shape parserãé¢åç­ç¥åäº§åé¢å port traitã?
- çæ­£æ§è¡ IOãè¿ç¨ãAI è°ç¨ãGit service è°ç¨ãå¹³å°éæãtool exposure æ?desktop/Tauri å·¥ä½ç?concrete adapter å±äºæ?crate å¤é¨ã?
- å¨ä¸æ¸¸è°ç¨ç¹è¢«ææè¿ç§»åï¼ç¨ re-export æ?wrapper facade ä¿ææ¢æ core import pathã?
- æ°å¢ feature-gated åå®¹å¿é¡»ä¿æçªè¾¹çã`miniapp`ã`function-agents` å?`product-full` åªåºå¯ç¨å·²å£°æçäº§åé¢å feature ç»ã?

## å½å±è¾¹ç

- `miniapp` å¯ä»¥æ¥æ MiniApp æ°æ®å½¢æãçº¯çå½å¨æå³ç­ãmetadata/import policyãbuilt-in bundle identityãembedded source assetsã?
  seed-plan factsãmarker wire formatãhost primitive call plan åçª portã?
- `function-agents` å¯ä»¥æ¥æ function-agent DTOãprompt/domain policyãresponse parsing/repair ruleãfile-shape analysis
  å?Git/AI port traitã?
- å·ä½ filesystem writesãmarker IOãhost dispatchãworker side effectãcompile orchestrationã`PathManager` integrationã?
  concrete Git/AI serviceãprovider acquisition å?transport error mapping åå±äºæ¬ crate å¤é¨ã?

## éªè¯

ææ¹å¨èå´éæ©æå°éªè¯ï¼

```bash
cargo test -p northhing-product-domains --no-default-features
cargo test -p northhing-product-domains --features product-full
node scripts/check-core-boundaries.mjs
cargo check -p northhing-core --features product-full
```

ä»æ¹ææ¡£æ¶è¿è¡?`git diff --check`ã?
