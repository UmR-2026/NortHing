**ä¸­æ** | [English](AGENTS.md)

# äº§åç»è£å±?

æ¬å±è´è´£äº§åç»è£ãå¼å®¹å¯¼åºãè½åéæ©å?runtime æ³¨åãå®ä¸ºä¸åäº¤ä»å½¢ææä¸å±è½åæ¥çº¿èµ·æ¥ï¼ä½ä¸æ¥æå·ä½?adapter è¡ä¸ºãå¯å¤ç¨ service å®ç°ãOS éææç¨³å®äº§åé¢åå¥çº¦ã?

## æ¨¡å

| Crate | èè´£ | æ¬å°ææ¡£ |
|---|---|---|
| `core` | `northhing-core` å¼å®¹é¨é¢ä¸?product-full ç»è£ | [AGENTS.md](core/AGENTS.md) |
| `product-capabilities` | äº§åè½å profileãtool group factsãservice requirements ä¸?harness selection | [AGENTS.md](product-capabilities/AGENTS.md) |

## æ¾ç½®è§å

- product-full æ¥çº¿ãå¼å®?shimãè½å?profile éæ©å?adapter/service æ³¨åæ¾å¨è¿éã?
- äº§åé¢åè§åå±äº `contracts/product-domains`ï¼ç»è£å±å¯ä»¥éæ©è¿äºäºå®ï¼ä½ä¸æ¥æå®ä»¬ã?
- ç¨³å® owner é»è¾ä¸ç§»å?`contracts`ï¼å¯ç§»æ¤æ§è¡é»è¾ä¸ç§»å?`execution`ï¼åè®®ééä¸ç§»å?`adapters`ï¼å¯å¤ç¨å®ç°ä¸ç§»å?`services`ã?
- ä¿æç°æ public import pathï¼é¤éè¿ç§»æç¡®ç§»é¤å¹¶è¡¥åå¼å®¹è¯´æåæµè¯ã?

## ä¾èµè¾¹ç

- `assembly/core` å¯ä»¥ä¾èµä¸å± owner æ¥ç»è£å½åäº§å?runtimeã?
- ç»è£å±å¯ä»¥ä¾èµ?adapter ä¸?service crateï¼ä½ä¸å®ç°å®ä»¬çåè®®åºååãè®¤è¯ãtransport æå¹³å°ç»èã?
- é¿åå¨ç»è£å±ç´æ¥ä½¿ç¨å®¿ä¸» APIï¼Tauri æ¯æå¿é¡»ä¿æ feature-gatedï¼å¹¶å°½å¯è½ç± app æ?adapter æ¥æã?
- interface crate å¯ä»¥è°ç¨ç»è£ APIï¼adapter å?service ä¸å¾ä¾èµç»è£å±ã?
