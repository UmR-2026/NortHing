// Daily Divination â?built-in MiniApp.
// Programmer-themed tarot: 24 cards, 4 fortune dimensions, daily-locked via app.storage.
//
// i18n strategy
// -------------
// Every locale-dependent dataset (cards / suits / colors / hours / mantras /
// insights / UI labels) is split into ZH and EN tables of equal length so the
// daily seed always picks the same *index* â?switching languages re-renders
// the same fortune in the chosen language without invalidating yesterday's
// stored "drawn" state. Visual fields (symbol/tone) are shared.

// ââ Cards: shared visuals + per-locale strings âââââââââââââââââââââââââââ
// Hue-balanced palette across 24 cards. Each `tone` is [primary, deep-bg] â?
// primary drives accents (fortune bars, scene tint), deep-bg is the card
// background gradient endpoint. Hues are spread roughly uniformly around the
// wheel (red â?orange â?gold â?lime â?teal â?cyan â?blue â?indigo â?violet
// â?magenta â?rose) while still nodding to each card's symbolism.
const CARD_VISUALS = [
  // 0  å½è¿ä¹è½® â?amethyst (270Â°)
  { symbol: 'â?, tone: ['#6d28d9', '#1a0936'] },
  // 1  æè¾°æå¼ â?sapphire (220Â°)
  { symbol: 'â?, tone: ['#1e3a8a', '#08112e'] },
  // 2  ççä¹å¿ â?molten orange (18Â°)
  { symbol: 'â?, tone: ['#c2410c', '#2a0a02'] },
  // 3  å¯éä¹é â?slate (215Â°, low-sat)
  { symbol: 'â?, tone: ['#475569', '#0c121b'] },
  // 4  é¶æ²³ä¹¦ç® â?deep indigo (250Â°)
  { symbol: 'â?, tone: ['#4338ca', '#0d0a2e'] },
  // 5  çº¢å®å äºº â?ruby (350Â°)
  { symbol: 'â?, tone: ['#be123c', '#2c0612'] },
  // 6  ééä¹è â?bronze (35Â°)
  { symbol: 'â?, tone: ['#92400e', '#261105'] },
  // 7  åä¹åå â?cyan (188Â°)
  { symbol: 'â?, tone: ['#0891b2', '#031f29'] },
  // 8  èèä½è¯­ â?moss (90Â°)
  { symbol: 'â', tone: ['#65a30d', '#121e02'] },
  // 9  ææµ·ç½ç â?steel blue (210Â°)
  { symbol: 'â?, tone: ['#1d4ed8', '#06163a'] },
  // 10 é»æçç« â?amber (28Â°)
  { symbol: 'â?, tone: ['#b45309', '#2a1106'] },
  // 11 æ¬æµ®ä¹ç¯ â?jade (170Â°)
  { symbol: 'â?, tone: ['#0f766e', '#03221f'] },
  // 12 éé¢æ¹?â?aqua (198Â°)
  { symbol: 'â?, tone: ['#0369a1', '#031c33'] },
  // 13 æ·±æä¿¡ä½¿ â?forest (135Â°)
  { symbol: 'â?, tone: ['#15803d', '#051a0d'] },
  // 14 å¤ä¹æç´ â?violet (285Â°)
  { symbol: 'â?, tone: ['#7e22ce', '#1c0830'] },
  // 15 é»æé¸é â?crimson (358Â°)
  { symbol: 'â?, tone: ['#b91c1c', '#260606'] },
  // 16 æåä¹çº± â?aurora teal-green (160Â°)
  { symbol: 'â?, tone: ['#0d9488', '#02322f'] },
  // 17 ç¾½è½ä¹ç¬ â?graphite (220Â°, near-neutral)
  { symbol: 'â?, tone: ['#52525b', '#0d0d10'] },
  // 18 æ½®æ±ä¹ç¯ â?ocean (235Â°)
  { symbol: 'â?, tone: ['#1e40af', '#08123a'] },
  // 19 ç´«æ¶å£æ¯ â?magenta (305Â°)
  { symbol: 'â?, tone: ['#a21caf', '#2a072d'] },
  // 20 éè²é½¿è½® â?gold (45Â°)
  { symbol: 'â?, tone: ['#a16207', '#2a1805'] },
  // 21 æ¨æ¦ä¹ç¿¼ â?rose (335Â°)
  { symbol: 'â?, tone: ['#be185d', '#2c0a1c'] },
  // 22 å¯æä¹å â?frost steel-cyan (200Â°, low-sat)
  { symbol: 'â?, tone: ['#0e7490', '#03161c'] },
  // 23 æåç³é¶ â?midnight (245Â°)
  { symbol: 'â?, tone: ['#312e81', '#0a0928'] },
];

const CARD_STRINGS = {
  'zh-CN': [
    { name: 'å½è¿ä¹è½®', tag: 'æºç¼', keyword: 'æµè½¬ Â· èå¥', quotes: [
      'æ¯ä¸ª commit é½å¨æ¹åå½è¿çæ²çï¼ä»å¤©å¼å¾ä¸æ¬¡æ¨éã?,
      'é½¿è½®èªæå¶è½¬æ³ï¼ä½ åªéå¨å¯¹çæ¶å»æä¸åè½¦ã?,
      'ä»æ¥å±äº"åå¨èµ·æ¥åè¯´"ï¼æ¹åä¼èªå·±æµ®ç°ã?,
      'æ¨å¤©å¡ä½çäºï¼æ¢ä¸ªæ¶é´ç¹åè¯ï¼å¸¸å¸¸å°±éäºã?,
    ] },
    { name: 'æè¾°æå¼', tag: 'å¸æ', keyword: 'è¿æ¹ Â· çµæ', quotes: [
      'å½ä½ å¡ä½æ¶ï¼æ¬å¤´çç documentation ä¹å¤çä¸çã?,
      'æç¼åæ¾è¿ä¸æ¡£ï¼ç¼åçæ­»ç»å°±æäºè·¯æ ã?,
      'ä»å¤©å¼å¾æ¶èä¸ç¯ä¸æ¥å¸¸é¡¹ç®æ å³çå¥½æã?,
      'ç¸ä¿¡é£ä¸ªè®©ä½ å¿å¨ç?å°å¯ä¸?å¿µå¤´ï¼å®å¨ä¸ºä½ å¯¼èªã?,
    ] },
    { name: 'ççä¹å¿', tag: 'é»é?, keyword: 'ç²¾ç¼ Â· éæ', quotes: [
      'ä»æ¥éåä¸æ¬¡ææ¢çéæï¼å é¤å³åé ã?,
      'ä½ å¿éé£æ®?æ©æè¦æ¹"çä»£ç ï¼ä»å¤©å°±æ¯æ©ã?,
      'ä¸å¶ä¿®è¡¥ï¼ä¸å¦æå®æ¨åçç«ééé¸ã?,
      'åæ³æ¯å æ³æ´éè¦åæ°ï¼ä»å¤©ä½ æè¿ä»½åæ°ã?,
    ] },
    { name: 'å¯éä¹é', tag: 'å¥æ³', keyword: 'æ·±æ?Â· æ²æ½', quotes: [
      'è®?IDE æåååéï¼ç­æ¡å¸¸å¨ç½æ¿ä¸æµ®ç°ã?,
      'ä»å¤©å°æå­ï¼å¤æ³ä¸æ³ãææä¼æè°¢å¤§èã?,
      'æé®é¢åä¸æ¥è¯»ä¸éï¼åæ° bug å½åºæ´é²ã?,
      'å®éæ¯æè¢«ä½ä¼°ççäº§åå·¥å·ã?,
    ] },
    { name: 'é¶æ²³ä¹¦ç®', tag: 'æºè¯', keyword: 'éè¯» Â· ç´¯ç§¯', quotes: [
      'ä»å¤©è¯»å®ä¸ä¸ªé¿ issue çè®¨è®ºï¼æ¯ååè¡ä»£ç å¼é±ã?,
      'åè®¸èªå·±è±ä¸å°æ¶è¯»æºç ï¼é£æ¯æ»éªççå¼å§ã?,
      'æ¶èå¤¹éé£ç¯æï¼ä»å¤©å°±è¯»å®å®ã?,
      'ä¸ç¯å¥½ç?RFCï¼èè¿åæ¬¡ä¼è®®ã?,
    ] },
    { name: 'çº¢å®å äºº', tag: 'åé?, keyword: 'éç¢ Â· ç»è', quotes: [
      'æä¸ä¸ªè¾¹çæ¡ä»¶æ³æ¸æ¥ï¼å°±æ¯ä»å¤©æå¥½çè¾åºã?,
      'ä»æ¥éåæç£¨é£ä¸ª"å·®ä¸å¤äº"çç»èã?,
      'éè¯¯ä¿¡æ¯ä¹æ¯äº§åçä¸é¨åï¼æå®åå¾äººè¯ä¸ç¹ã?,
      'ä¸å¤å¾®è°ï¼å¾å¾èè¿ä¸æ¬¡éåã?,
    ] },
    { name: 'ééä¹è', tag: 'èå', keyword: 'ç¯è·¯ Â· èå', quotes: [
      'ä¸ä¸?retry-loop ä¿®å¥½äºï¼æ´æ¡é¾è·¯é½æ´»äºè¿æ¥ã?,
      'è®©èªå·±ç»åä¸æ¬?åæ¥å¦æ­¤"çç¬é´ã?,
      'ä»å¤©å¼å¾ä¸æ¬¡å½»åºçè®¤ç¥å·æ°ã?,
      'æ¢ä¸ªè§åº¦çé£ä¸ªèé®é¢ï¼å®ä¼åå¾å¾å°ã?,
    ] },
    { name: 'åä¹åå', tag: 'åä½', keyword: 'åå£° Â· å±æ¯', quotes: [
      'ä¸å?ææ¥å¸®ä½ çç"ï¼å°±æ¯ä»æ¥æå¼ºç buffã?,
      'ä¸»å¨ ping ä¸ä¸å¡ä½çåäºï¼ä½ ç?5 åéå¯è½çä»åå¤©ã?,
      'ä»å¤©ç­ä¸ä¸ªå«äººé®è¿ä½ çé®é¢ï¼åå£°ä¼ä¼ å¾å¾è¿ã?,
      'æè°¢ä¸ä½å¸®è¿ä½ çåäºï¼è¶å·ä½è¶å¥½ã?,
    ] },
    { name: 'èèä½è¯­', tag: 'ä¼æ©', keyword: 'çé¿ Â· çç½', quotes: [
      'è®©è¿åº¦æ¡æ¢ä¸ç¹ï¼è®©åé åå¿«ä¸ç¹ã?,
      'ä»æ¥å®å·ä¸ä¼å¿æï¼çµæä¸å¨é®çä¸ã?,
      'åè®¸ä¸å¤©ç"çä¼¼æ²¡äº§å?ï¼åå£¤éè¦æ¶é´åéµã?,
      'ææ¤å­æ¨å¼ï¼å»çªè¾¹ç«ä¸åéã?,
    ] },
    { name: 'ææµ·ç½ç', tag: 'ææ©', keyword: 'æ¹å Â· å³æ­', quotes: [
      'å«åçº ç»ææ¯éåï¼åæç¬¬ä¸è¡ä»£ç ååºæ¥ã?,
      'ä»æ¥éåååºé£ä¸ªä¸ç´æççå³å®ã?,
      'é?A è¿æ¯é?B é½è¡ï¼åªè¦å«åé?åç­ç­?ã?,
      'ææ¹æ¡åå¨çº¸ä¸ï¼å¤æ°éæ©ä¼èªææ­æã?,
    ] },
    { name: 'é»æçç«', tag: 'ä¸æ³¨', keyword: 'å¿æµ Â· çç§', quotes: [
      'å³é­ Slackï¼ä»å¤©å±äºä½ åç¼è¾å¨çäºäººä¸çã?,
      'æä»å¤©ææ³åçäºæå°ä¸åç¬¬ä¸æ ¼ã?,
      'ä¸æ®µä¸è¢«ææ­ç 90 åéï¼èè¿ä¸æ´å¤©çç¢çæ¶é´ã?,
      'è®?å¿æ°æ¨¡å¼"æä¸ºä»å¤©çç¤¼ç©ã?,
    ] },
    { name: 'æ¬æµ®ä¹ç¯', tag: 'å¹³è¡¡', keyword: 'åè Â· å¼ å', quotes: [
      'å®ç¾ä¸ä¸çº¿ä¹é´ï¼è¯·éæ©ä¸çº¿ã?,
      'ä»å¤©å¼å¾ä¸ºæä»¶äºè¯´ä¸æ¬?ä¸?ã?,
      'å°åä¸ä»¶äºï¼è¿æ¯å¤åä¸ä»¶äºé¾ã?,
      'æèå´ç¼©å°ä¸åï¼ææå¸¸å¸¸ç¿»åã?,
    ] },
    { name: 'éé¢æ¹?, tag: 'å¤ç', keyword: 'æ ç§ Â· è§å¯', quotes: [
      'åçä¸å¨åèªå·±åçä»£ç ï¼ä¼æ¯?review æ´è¯å®ã?,
      'ä»å¤©åä¸æ®µä¸è¡çå¤çï¼æå¤©å°±ç¨å¾å°ã?,
      'é®èªå·±ï¼è¿ä¸å¨æè®©æèªè±ªçä¸ä»¶äºæ¯ä»ä¹ï¼',
      'è¿å»çä½ ç¯è¿çéï¼æªå¿ä½ ä»å¤©è¿å¨ç¯ã?,
    ] },
    { name: 'æ·±æä¿¡ä½¿', tag: 'æ¶æ¯', keyword: 'ä¼ è¾¾ Â· é¾æ¥', quotes: [
      'ä¸å°åå¾æ¸æ¥çé®ä»¶ï¼èè¿ä¸åºä¼è®®ã?,
      'ä»å¤©éåä¸»å¨åæ­¥ä¸æ¬¡è¿å±ï¼è®©ä¿¡æ¯èµ°å¨åé¢ã?,
      'æé£æ¡æ³äºä¸å¤©çè¯ååºå»ï¼æåä¸è¿æ²¡åå¤ã?,
      'ä¸å?å¯¹é½ä¸ä¸?ï¼è½çæä¸å¨ççæµã?,
    ] },
    { name: 'å¤ä¹æç´', tag: 'è¯æ', keyword: 'éµå¾ Â· ä¼é', quotes: [
      'ä¸ºåéèµ·ä¸ä¸ªå¨å¬çåå­ï¼å½åæ¯ç¨åºåçè¯ã?,
      'ä»å¤©åä¸æ®µä½ æ¿ææ¿ç»æåççä»£ç ã?,
      'è®©å½æ°åå¥å­é£æ ·æè¯»ï¼è®©æ¨¡ååæ®µè½é£æ ·èªæ´½ã?,
      'æç©ºè¡ç¨å¾åå¼å¸ä¸æ ·èªç¶ã?,
    ] },
    { name: 'é»æé¸é', tag: 'åæ°', keyword: 'ç´é¢ Â· ææ', quotes: [
      'ä»å¤©ç´é¢é£ä¸ªä¸ç´è¢«ä½ è·³è¿ç TODOã?,
      'ææé¾çé£ä»¶äºæ¾å¨ç¬¬ä¸ä¸ªï¼å©ä¸çä¼åå®¹æã?,
      'è¯¥è¯´çè¯å°±è¯´åºæ¥ï¼è¿å°çåé¦æ¯æ²¡ç¤¼è²çåé¦ã?,
      'æ?ç­æå­¦ä¼åå"æ¢æ"è¾¹åè¾¹å­¦"ã?,
    ] },
    { name: 'æåä¹çº±', tag: 'çµæ', keyword: 'è¿¸å Â· æµå¨', quotes: [
      'ä¿ææ²æµ´ææ£æ­¥çç¶æï¼bug å¤åå¨æ°´æµå£°éè¢«å²æã?,
      'ä»æ¥çå¥½ç¹å­å¨é®çå¤ï¼è®°å¾å¸¦ä¸ªæ¬å­ã?,
      'åè®¸èªå·±ææ¶ç¦»å¼å±å¹ï¼çµæä¼ä»èåè¿½ä¸æ¥ã?,
      'æ¢ä¸ä¸ªåä»£ç çå°æ¹ï¼æè·¯ä¹ä¼è·çæªçªã?,
    ] },
    { name: 'ç¾½è½ä¹ç¬', tag: 'è®°å½', keyword: 'ä¹¦å Â· æ²æ·', quotes: [
      'ä»æ¥éååä¸ç¯ææ¡£ï¼æªæ¥çä½ ä¼æè°¢ç°å¨çèªå·±ã?,
      'æå£å£ç¸ä¼ çè§åè½å° README éã?,
      'ä¸ºä»å¤©çå°å³å®åä¸å?ä¸ºä»ä¹?ï¼åå¹´åå®æä½ ã?,
      'æèå­éçå¾ç»å° README éï¼å¢éå°±æäºå±è¯ã?,
    ] },
    { name: 'æ½®æ±ä¹ç¯', tag: 'èå¥', keyword: 'èµ·ä¼ Â· å¨æ', quotes: [
      'é«æä¸ä½è°·çæ¯æ½®æ±ï¼éè¦çæ¯å«å¨éæ½®æ¶è´£æªèªå·±ã?,
      'ä»æ¥å®è·çèº«ä½èµ°ï¼æçèªæå¶æ½®ä½ã?,
      'ä¸å¿æ¯å¤©é½å¨åå¥è·ï¼ä¼è·çäººä¹ä¼èµ°ã?,
      'ä½è½éæ¶æ®µï¼åä½è½éä»»å¡ï¼é£å«èªæã?,
    ] },
    { name: 'ç´«æ¶å£æ¯', tag: 'ä¸°é¥¶', keyword: 'æ»å» Â· é¦èµ ', quotes: [
      'å«å¿äºåæ°´ãä¹å«å¿äºå¤¸èªå·±ä¸å¥ã?,
      'ä»æ¥ç»èªå·±çä¸ä»½å°å¥å±ï¼åªææ¯ä¸æ¯å¥½åå¡ã?,
      'åé¡¿å¥½çï¼ååå» debugã?,
      'ä»å¤©å¯¹èªå·±æ¸©æä¸äºï¼ä¸çå¯¹ä½ ä¹ä¼ã?,
    ] },
    { name: 'éè²é½¿è½®', tag: 'ç³»ç»', keyword: 'æºå¶ Â· æ¶æ', quotes: [
      'ä¸ä¸ªæ¸æ°çæ¨¡åè¾¹çï¼èè¿åä¸ªèªæç hackã?,
      'ä»æ¥å®ç»ä¸å¼ æ¶æå¾ï¼å¨èå­ä¹å¤æå®æ¾å½¢ã?,
      'ä¸å¶æè¡¥ä¸ï¼ä¸å¦åæ³æ¸æ¥æ¯è°å¨åè°è¯´è¯ã?,
      'ä¸ºæºå¶æèµä¸ç¹æ¶é´ï¼æªæ¥è¿æ¬å¸¦å©è¿ä½ ã?,
    ] },
    { name: 'æ¨æ¦ä¹ç¿¼', tag: 'å¯ç¨', keyword: 'åºå Â· ç¬¬ä¸æ­?, quotes: [
      'æ?ç­æåå¤å¥?æ¢æ"å?push ä¸ä¸?draft PR"ã?,
      'ä»æ¥éåå¼ä¸ä¸ªæ°ä»åºï¼åªæåªåä¸ä¸?READMEã?,
      '0 â?1 æ°¸è¿æ¯æé¾ä¹æå¼å¾çé£ä¸æ­¥ã?,
      'åªè¦å¼å§ï¼å°±å·²ç»é¢åæ¨å¤©çèªå·±ã?,
    ] },
    { name: 'å¯æä¹å', tag: 'æ¸ç®', keyword: 'åé¤ Â· åå?, quotes: [
      'ä»å¤©éåå ä¸äºè¿æ¶çä¾èµï¼å°å³æ¯å¤ã?,
      'æé£ä¸ªä¸å¹´æ²¡äººç¨çåè½ä¸çº¿å§ã?,
      'æ¶ä»¶ç®±æ¸é¶ä¸æ¬¡ï¼æ´ä¸ªäººé½è½»çäºã?,
      'è¿æçå¾åï¼ä¸å å°±æ¯å¨å·æªæ¥ä½ çæ³¨æåã?,
    ] },
    { name: 'æåç³é¶', tag: 'æå¼', keyword: 'å¤è¡ Â· æ­¥æ­¥', quotes: [
      'ä¸å¿çæ¸æ´ä¸ªé¶æ¢¯ï¼åè¿åºç¼åçè¿ä¸æ­¥ã?,
      'ä»æ¥åªé®"ä¸ä¸å°æ­¥æ¯ä»ä¹?ï¼å«çäº¤ç»æå¤©ã?,
      'é»æéèµ°å¾ç¨³çäººï¼é½ä¸é çæ¸è¿æ¹ã?,
      'æå¤§ç®æ æå° 30 åéä»¥åï¼åå¼å§å¨æã?,
    ] },
  ],
  'zh-TW': [
    { name: 'å½éä¹è¼ª', tag: 'æ©ç·£', keyword: 'æµè½ Â· ç¯å¥?, quotes: [
      'æ¯å?commit é½å¨æ¹è®å½éçæ²çï¼ä»å¤©å¼å¾ä¸æ¬¡æ¨éã?,
      'é½è¼ªèªæå¶è½æ³ï¼ä½ åªéå¨å°çæå»æä¸åè»ã?,
      'ä»æ¥å±¬æ¼"ååèµ·ä¾åèªª"ï¼æ¹åæèªå·±æµ®ç¾ã?,
      'æ¨å¤©å¡ä½çäºï¼æåæéé»åè©¦ï¼å¸¸å¸¸å°±éäºã?,
    ] },
    { name: 'æè¾°æå¼', tag: 'å¸æ', keyword: 'é æ¹ Â· éæ', quotes: [
      'ç¶ä½ å¡ä½æï¼æ¬é ­çç documentation ä¹å¤çä¸çã?,
      'æç¼åæ¾é ä¸æªï¼ç¼åçæ­»çµå°±æäºè·¯æ¨ã?,
      'ä»å¤©å¼å¾æ¶èä¸ç¯èæ¥å¸¸é ç®ç¡éçå¥½æã?,
      'ç¸ä¿¡é£åè®ä½ å¿åç"å°å¯æ¥?å¿µé ­ï¼å®å¨çºä½ å°èªã?,
    ] },
    { name: 'ççä¹å¿', tag: 'éé?, keyword: 'ç²¾ç Â· éæ§', quotes: [
      'ä»æ¥é©åä¸æ¬¡ææ¢çéæ§ï¼åªé¤å³åµé ã?,
      'ä½ å¿è£¡é£æ®?æ©æè¦æ¹"çä»£ç¢¼ï¼ä»å¤©å°±æ¯æ©ã?,
      'èå¶ä¿®è£ï¼ä¸å¦æå®æ¨åçç«è£¡ééã?,
      'æ¸æ³æ¯å æ³æ´éè¦åæ°£ï¼ä»å¤©ä½ æéä»½åæ°£ã?,
    ] },
    { name: 'å¯éä¹é', tag: 'å¥æ³', keyword: 'æ·±æ?Â· æ²æ½', quotes: [
      'è®?IDE æ«åååéï¼ç­æ¡å¸¸å¨ç½æ¿ä¸æµ®ç¾ã?,
      'ä»å¤©å°æå­ï¼å¤æ³ä¸æ³ãææææè¬å¤§è¦ã?,
      'æåé¡å¯«ä¸ä¾è®ä¸éï¼åæ¸ bug ç¶å ´æ´é²ã?,
      'å®éæ¯æè¢«ä½ä¼°ççç¢åå·¥å·ã?,
    ] },
    { name: 'éæ²³æ¸ç°?, tag: 'æºè­', keyword: 'é±è® Â· ç´¯ç©', quotes: [
      'ä»å¤©è®å®ä¸åé· issue çè¨è«ï¼æ¯å¯«åè¡ä»£ç¢¼å¼é¢ã?,
      'åè¨±èªå·±è±ä¸å°æè®æºç¢¼ï¼é£æ¯æ»¾éªççéå§ã?,
      'æ¶èå¤¾è£¡é£ç¯æï¼ä»å¤©å°±è®å®å®ã?,
      'ä¸ç¯å¥½ç?RFCï¼åéåæ¬¡æè­°ã?,
    ] },
    { name: 'ç´å¯¶å äºº', tag: 'åµé?, keyword: 'éç¢ Â· ç´°ç¯', quotes: [
      'æä¸åéçæ¢ä»¶æ³æ¸æ¥ï¼å°±æ¯ä»å¤©æå¥½çè¼¸åºã?,
      'ä»æ¥é©åæç£¨é£å?å·®ä¸å¤äº"çç´°ç¯ã?,
      'é¯èª¤ä¿¡æ¯ä¹æ¯ç¢åçä¸é¨åï¼æå®å¯«å¾äººè©±ä¸é»ã?,
      'ä¸èå¾®èª¿ï¼å¾å¾åéä¸æ¬¡éå¯«ã?,
    ] },
    { name: 'ééä¹è', tag: 'è»è®', keyword: 'ç°è·¯ Â· è»è®', quotes: [
      'ä¸å?retry-loop ä¿®å¥½äºï¼æ´æ¢éè·¯é½æ´»äºéä¾ã?,
      'è®èªå·±ç¶æ­·ä¸æ¬?åä¾å¦æ­¤"çç¬éã?,
      'ä»å¤©å¼å¾ä¸æ¬¡å¾¹åºçèªç¥å·æ°ã?,
      'æåè§åº¦çé£åèåé¡ï¼å®æè®å¾å¾å°ã?,
    ] },
    { name: 'åä¹è¿´é¿', tag: 'åä½', keyword: 'åè² Â· å±æ¯', quotes: [
      'ä¸å?æä¾å¹«ä½ çç"ï¼å°±æ¯ä»æ¥æå¼·ç buffã?,
      'ä¸»å ping ä¸ä¸å¡ä½çåäºï¼ä½ ç?5 åéå¯è½çä»åå¤©ã?,
      'ä»å¤©ç­ä¸åå¥äººåéä½ çåé¡ï¼åè²æå³å¾å¾é ã?,
      'æè¬ä¸ä½å¹«éä½ çåäºï¼è¶å·é«è¶å¥½ã?,
    ] },
    { name: 'èèä½èª', tag: 'ä¼æ©', keyword: 'çé· Â· çç½', quotes: [
      'è®é²åº¦æ¢æ¢ä¸é»ï¼è®åµé åå¿«ä¸é»ã?,
      'ä»æ¥å®å·ä¸æåæ¶ï¼éæä¸å¨éµç¤ä¸ã?,
      'åè¨±ä¸å¤©ç"çä¼¼æ²ç¢å?ï¼åå£¤éè¦æéç¼éµã?,
      'ææ¤å­æ¨éï¼å»çªéç«ä¸åéã?,
    ] },
    { name: 'ææµ·ç¾ç¤', tag: 'ææ', keyword: 'æ¹å Â· æ±ºæ·', quotes: [
      'å¥åç³¾çµæè¡é¸åï¼åæç¬¬ä¸è¡ä»£ç¢¼å¯«åºä¾ã?,
      'ä»æ¥é©åååºé£åä¸ç´æèçæ±ºå®ã?,
      'é?A éæ¯é?B é½è¡ï¼åªè¦å¥åé¸"åç­ç­?ã?,
      'ææ¹æ¡å¯«å¨ç´ä¸ï¼å¤æ¸é¸ææèªææ­æã?,
    ] },
    { name: 'é»æçç«', tag: 'å°æ³¨', keyword: 'å¿æµ Â· çç', quotes: [
      'éé Slackï¼ä»å¤©å±¬æ¼ä½ åç·¨è¼¯å¨çäºäººä¸çã?,
      'æä»å¤©ææ³åçäºæå°ä¸åç¬¬ä¸æ ¼ã?,
      'ä¸æ®µä¸è¢«ææ·ç 90 åéï¼åéä¸æ´å¤©çç¢çæéã?,
      'è®?å¿æ¾æ¨¡å¼"æçºä»å¤©çç¦®ç©ã?,
    ] },
    { name: 'æ¸æµ®ä¹ç°', tag: 'å¹³è¡¡', keyword: 'åæ¨ Â· å¼µå', quotes: [
      'å®ç¾èä¸ç·ä¹éï¼è«é¸æä¸ç·ã?,
      'ä»å¤©å¼å¾çºæä»¶äºèªªä¸æ¬?ä¸?ã?,
      'å°åä¸ä»¶äºï¼é æ¯å¤åä¸ä»¶äºé£ã?,
      'æç¯åç¸®å°ä¸åï¼ææå¸¸å¸¸ç¿»åã?,
    ] },
    { name: 'é¡é¢æ¹?, tag: 'è¦ç¤', keyword: 'æ ç§ Â· è¦ºå¯', quotes: [
      'åçä¸é±åèªå·±å¯«çä»£ç¢¼ï¼ææ¯?review æ´èª å¯¦ã?,
      'ä»å¤©å¯«ä¸æ®µä¸è¡çè¦ç¤ï¼æå¤©å°±ç¨å¾å°ã?,
      'åèªå·±ï¼éä¸é±æè®æèªè±ªçä¸ä»¶äºæ¯ä»éº¼ï¼',
      'éå»çä½ ç¯éçé¯ï¼æªå¿ä½ ä»å¤©éå¨ç¯ã?,
    ] },
    { name: 'æ·±æä¿¡ä½¿', tag: 'æ¶æ¯', keyword: 'å³é Â· éæ¥', quotes: [
      'ä¸å°å¯«å¾æ¸æ¥çéµä»¶ï¼åéä¸å ´æè­°ã?,
      'ä»å¤©é©åä¸»ååæ­¥ä¸æ¬¡é²å±ï¼è®ä¿¡æ¯èµ°å¨åé¢ã?,
      'æé£æ¢æ³äºä¸å¤©çè©±ç¼åºå»ï¼æå£ä¸éæ²åå¾©ã?,
      'ä¸å?å°é½ä¸ä¸?ï¼è½çæä¸é±ççæ¸¬ã?,
    ] },
    { name: 'å¤ä¹æç´', tag: 'è©©æ', keyword: 'é»å¾ Â· åªé', quotes: [
      'çºè®éèµ·ä¸ååè½çåå­ï¼å½åæ¯ç¨åºå¡çè©©ã?,
      'ä»å¤©å¯«ä¸æ®µä½ é¡ææ¿çµ¦æåççä»£ç¢¼ã?,
      'è®å½æ¸åå¥å­é£æ¨£æè®ï¼è®æ¨¡å¡åæ®µè½é£æ¨£èªæ´½ã?,
      'æç©ºè¡ç¨å¾åå¼å¸ä¸æ¨£èªç¶ã?,
    ] },
    { name: 'é»æééµ', tag: 'åæ°£', keyword: 'ç´é¢ Â· ææ°', quotes: [
      'ä»å¤©ç´é¢é£åä¸ç´è¢«ä½ è·³éç TODOã?,
      'ææé£çé£ä»¶äºæ¾å¨ç¬¬ä¸åï¼å©ä¸çæè®å®¹æã?,
      'è©²èªªçè©±å°±èªªåºä¾ï¼é²å°çåé¥æ¯æ²ç¦®è²çåé¥ã?,
      'æ?ç­æå­¸æåå"ææ"éåéå­¸"ã?,
    ] },
    { name: 'æ¥µåä¹ç´', tag: 'éæ', keyword: 'è¿¸ç¼ Â· æµå', quotes: [
      'ä¿ææ²æµ´ææ£æ­¥ççæï¼bug å¤åå¨æ°´æµè²è£¡è¢«æ²æã?,
      'ä»æ¥çå¥½é»å­å¨éµç¤å¤ï¼è¨å¾å¸¶åæ¬å­ã?,
      'åè¨±èªå·±æ«æé¢éå±å¹ï¼éææå¾èå¾è¿½ä¸ä¾ã?,
      'æä¸åå¯«ä»£ç¢¼çå°æ¹ï¼æè·¯ä¹æè·èæªçª©ã?,
    ] },
    { name: 'ç¾½è½ä¹ç­', tag: 'è¨é', keyword: 'æ¸å¯« Â· æ²æ¾±', quotes: [
      'ä»æ¥é©åå¯«ä¸ç¯ææªï¼æªä¾çä½ ææè¬ç¾å¨çèªå·±ã?,
      'æå£å£ç¸å³çè¦åè½å° README è£¡ã?,
      'çºä»å¤©çå°æ±ºå®å¯«ä¸å?çºä»éº?ï¼åå¹´å¾å®æä½ ã?,
      'æè¦å­è£¡çåç«å° README è£¡ï¼åéå°±æäºå±è­ã?,
    ] },
    { name: 'æ½®æ±ä¹ç°', tag: 'ç¯å¥?, keyword: 'èµ·ä¼ Â· é±æ', quotes: [
      'é«æèä½è°·çæ¯æ½®æ±ï¼éè¦çæ¯å¥å¨éæ½®æè²¬æªèªå·±ã?,
      'ä»æ¥å®è·èèº«é«èµ°ï¼æçèªæå¶æ½®ä½ã?,
      'ä¸å¿æ¯å¤©é½å¨åå¥è·ï¼æè·çäººä¹æèµ°ã?,
      'ä½è½éææ®µï¼åä½è½éä»»åï¼é£å«è°æã?,
    ] },
    { name: 'ç´«æ¶èç', tag: 'è±é¥', keyword: 'æ»é¤ Â· é¥è´', quotes: [
      'å¥å¿äºåæ°´ãä¹å¥å¿äºèªèªå·±ä¸å¥ã?,
      'ä»æ¥çµ¦èªå·±çä¸ä»½å°çåµï¼åªææ¯ä¸æ¯å¥½åå¡ã?,
      'åé å¥½çï¼ååå» debugã?,
      'ä»å¤©å°èªå·±æº«æä¸äºï¼ä¸çå°ä½ ä¹æã?,
    ] },
    { name: 'éè²é½è¼ª', tag: 'ç³»çµ±', keyword: 'æ©å¶ Â· æ¶æ§', quotes: [
      'ä¸åæ¸æ°çæ¨¡å¡éçï¼åéååè°æç hackã?,
      'ä»æ¥å®ç«ä¸å¼µæ¶æ§åï¼å¨è¦å­ä¹å¤æå®é¡¯å½¢ã?,
      'èå¶æè£ä¸ï¼ä¸å¦åæ³æ¸æ¥æ¯èª°å¨åèª°èªªè©±ã?,
      'çºæ©å¶æè³ä¸é»æéï¼æªä¾é£æ¬å¸¶å©éä½ ã?,
    ] },
    { name: 'æ¨æ¦ä¹ç¿¼', tag: 'åç¨', keyword: 'åºç¼ Â· ç¬¬ä¸æ­?, quotes: [
      'æ?ç­ææºåå¥?ææ"å?push ä¸å?draft PR"ã?,
      'ä»æ¥é©åéä¸åæ°ååº«ï¼åªæåªå¯«ä¸å?READMEã?,
      '0 â?1 æ°¸é æ¯æé£ä¹æå¼å¾çé£ä¸æ­¥ã?,
      'åªè¦éå§ï¼å°±å·²ç¶é åæ¨å¤©çèªå·±ã?,
    ] },
    { name: 'å¯æä¹å', tag: 'æ¸ç®', keyword: 'åé¤ Â· æ·¨å', quotes: [
      'ä»å¤©é©ååªä¸äºéæçä¾è³´ï¼å°å³æ¯å¤ã?,
      'æé£åä¸å¹´æ²äººç¨çåè½ä¸ç·å§ã?,
      'æ¶ä»¶ç®±æ¸é¶ä¸æ¬¡ï¼æ´åäººé½è¼çäºã?,
      'éæçå¾è¾¦ï¼ä¸åªå°±æ¯å¨å·æªä¾ä½ çæ³¨æåã?,
    ] },
    { name: 'æåç³é', tag: 'æå¼', keyword: 'å¤è¡ Â· æ­¥æ­¥', quotes: [
      'ä¸å¿çæ¸æ´åéæ¢¯ï¼åéåºç¼åçéä¸æ­¥ã?,
      'ä»æ¥åªå"ä¸ä¸å°æ­¥æ¯ä»éº?ï¼å¥çäº¤çµ¦æå¤©ã?,
      'é»æè£¡èµ°å¾ç©©çäººï¼é½ä¸é çæ¸é æ¹ã?,
      'æå¤§ç®æ¨æå° 30 åéä»¥å§ï¼åéå§åæã?,
    ] },
  ],

  'en-US': [
    { name: 'Wheel of Fortune', tag: 'Chance', keyword: 'Flow Â· Rhythm', quotes: [
      'Every commit bends the curve of fate â?today is worth a push.',
      'The gears spin themselves; you just press Enter at the right moment.',
      'Today belongs to "start moving"; direction will reveal itself.',
      'What blocked you yesterday often unblocks itself at a different hour.',
    ] },
    { name: 'Star Compass', tag: 'Hope', keyword: 'Distance Â· Inspiration', quotes: [
      'When stuck, look beyond the documentation.',
      'Zoom out one notch â?the knot turns into a signpost.',
      'Save one good article unrelated to today\'s project.',
      'Trust that little side-project itch; it knows where to take you.',
    ] },
    { name: 'Heart of the Forge', tag: 'Forge', keyword: 'Refine Â· Refactor', quotes: [
      'Today rewards a brave refactor â?deletion is creation.',
      'That code you swore you\'d fix "someday" â?today is someday.',
      'Stop patching; cast it back into the fire and reforge it.',
      'Subtraction takes more courage than addition; today you have it.',
    ] },
    { name: 'Silent Bell', tag: 'Meditate', keyword: 'Reflect Â· Sink', quotes: [
      'Pause your IDE for ten minutes â?answers surface on the whiteboard.',
      'Type less, think more. Your fingers will thank your brain.',
      'Write the problem down and read it once â?half the bugs reveal themselves.',
      'Quiet is the most underrated productivity tool.',
    ] },
    { name: 'Galactic Codex', tag: 'Knowledge', keyword: 'Read Â· Compound', quotes: [
      'Reading one long issue thread today beats writing ten lines of code.',
      'Allow yourself an hour of source-reading â?that\'s how the snowball starts.',
      'That tab in your "read later" â?finish it today.',
      'A good RFC beats ten meetings.',
    ] },
    { name: 'Ruby Artisan', tag: 'Craft', keyword: 'Polish Â· Detail', quotes: [
      'Thinking one edge case through clearly is today\'s best output.',
      'Polish the detail you\'ve been calling "good enough".',
      'Error messages are part of the product â?write them like a human.',
      'One small tweak often beats one full rewrite.',
    ] },
    { name: 'Bronze Serpent', tag: 'Shed', keyword: 'Loop Â· Renewal', quotes: [
      'Fix one retry-loop and the whole pipeline comes back to life.',
      'Let yourself have one "oh, that\'s why" moment today.',
      'Today deserves a real cognitive refresh.',
      'View that old problem from another angle â?it shrinks.',
    ] },
    { name: 'Echo of Light', tag: 'Collab', keyword: 'Echo Â· Resonance', quotes: [
      '"Let me take a look" is today\'s strongest buff.',
      'Ping a stuck teammate â?your 5 minutes may save their afternoon.',
      'Answer a question someone once asked you; the echo travels far.',
      'Thank someone who helped you â?the more specific, the better.',
    ] },
    { name: 'Moss Whispers', tag: 'Rest', keyword: 'Grow Â· Whitespace', quotes: [
      'Slow the progress bar, speed the imagination.',
      'Today permits a little laziness â?inspiration isn\'t on the keyboard.',
      'Allow a day that "looks unproductive" â?soil needs time to ferment.',
      'Push the chair back; stand by the window for three minutes.',
    ] },
    { name: 'Astrolabe', tag: 'Decide', keyword: 'Direction Â· Resolve', quotes: [
      'Stop agonizing over stack choices â?write line one first.',
      'Today is a good day to make that decision you\'ve been postponing.',
      'A or B is fine â?just stop choosing "wait a bit longer".',
      'Write the options on paper; most choices unmask themselves.',
    ] },
    { name: 'Dusk Hearth', tag: 'Focus', keyword: 'Flow Â· Burn', quotes: [
      'Close Slack â?today belongs to you and your editor.',
      'Put your most important task in the first slot of the morning.',
      'Ninety unbroken minutes beat a whole day of fragments.',
      'Let "Do Not Disturb" be today\'s gift to yourself.',
    ] },
    { name: 'Floating Ring', tag: 'Balance', keyword: 'Trade Â· Tension', quotes: [
      'Between perfect and shipped, choose shipped.',
      'Today is worth saying "no" to one thing.',
      'Doing one thing less is harder than doing one thing more.',
      'Halve the scope and the impact often doubles.',
    ] },
    { name: 'Mirror Lake', tag: 'Reflect', keyword: 'Reflect Â· Awareness', quotes: [
      'Re-reading code from a week ago is more honest than any review.',
      'Write a three-line retro today; tomorrow will use it.',
      'Ask yourself: what am I most proud of this week?',
      'Mistakes the past you made â?today you may already be past them.',
    ] },
    { name: 'Forest Courier', tag: 'Message', keyword: 'Convey Â· Connect', quotes: [
      'One clearly written email beats three meetings.',
      'Sync progress proactively; let information run ahead.',
      'Send the message you\'ve been drafting for three days â?silence is the worst case.',
      'A simple "let\'s align" saves a week of guessing.',
    ] },
    { name: 'Night Violin', tag: 'Poetic', keyword: 'Cadence Â· Grace', quotes: [
      'Give a variable a beautiful name â?naming is the programmer\'s poetry.',
      'Today, write code you\'d show a friend.',
      'Make functions read like sentences and modules cohere like paragraphs.',
      'Use blank lines as naturally as breath.',
    ] },
    { name: 'Dawn Iron', tag: 'Courage', keyword: 'Face Â· Challenge', quotes: [
      'Face the TODO you\'ve been skipping.',
      'Put the hardest task first; the rest become easier.',
      'Say the thing â?late feedback is rude feedback.',
      'Replace "after I learn it" with "learn while doing".',
    ] },
    { name: 'Aurora Veil', tag: 'Inspire', keyword: 'Burst Â· Flow', quotes: [
      'Take a shower or a walk â?most bugs wash away in running water.',
      'Today\'s best ideas are off the keyboard; bring a notebook.',
      'Let yourself leave the screen; inspiration catches up from behind.',
      'Change where you code and your thinking changes too.',
    ] },
    { name: 'Feather Quill', tag: 'Record', keyword: 'Write Â· Settle', quotes: [
      'Today is for writing a doc â?future-you will be grateful.',
      'Move tribal knowledge into the README.',
      'Add one "why" to today\'s small decision; six months later it saves you.',
      'Draw the picture in your head into the README; the team gets shared truth.',
    ] },
    { name: 'Tidal Ring', tag: 'Rhythm', keyword: 'Ebb Â· Cycle', quotes: [
      'Both peaks and troughs are tides â?don\'t blame yourself at low tide.',
      'Today, follow your body; productivity has its own waterline.',
      'You don\'t need to sprint every day; the best runners also walk.',
      'Match low-energy hours with low-energy tasks â?that\'s being smart.',
    ] },
    { name: 'Amethyst Chalice', tag: 'Bounty', keyword: 'Nourish Â· Gift', quotes: [
      'Don\'t forget to drink water. Or to praise yourself.',
      'Leave yourself a small reward today, even just a great coffee.',
      'Eat well, then go back to debugging.',
      'Be gentle with yourself today; the world will return the favor.',
    ] },
    { name: 'Golden Gear', tag: 'System', keyword: 'Mechanism Â· Architecture', quotes: [
      'A clear module boundary beats ten clever hacks.',
      'Today, draw an architecture diagram; make it real outside your head.',
      'Before patching, ask who is talking to whom.',
      'Invest in mechanism; the future repays with interest.',
    ] },
    { name: 'Dawn Wings', tag: 'Begin', keyword: 'Depart Â· First step', quotes: [
      'Replace "when I\'m ready" with "open a draft PR".',
      'Today is for starting a new repo â?even just a README.',
      '0 â?1 is always the hardest and most worthwhile step.',
      'The moment you start, you\'re already ahead of yesterday.',
    ] },
    { name: 'Frost Star Blade', tag: 'Purge', keyword: 'Prune Â· Cleanse', quotes: [
      'Today is for deleting outdated dependencies â?less is more.',
      'Sunset that feature no one has used in a year.',
      'Inbox-zero once and your whole self feels lighter.',
      'Stale TODOs steal future-you\'s attention; delete them.',
    ] },
    { name: 'Moonlit Steps', tag: 'Guide', keyword: 'Night walk Â· Step', quotes: [
      'You don\'t need to see the whole staircase â?just take the next step.',
      'Today, only ask "what is the next small step"; leave the rest to tomorrow.',
      'Those who walk steadily in the dark don\'t depend on seeing far.',
      'Cut big goals into 30-minute slices, then begin.',
    ] },
  ],
};

const FORTUNE_KEY_IDS = ['overall', 'work', 'inspire', 'wealth'];

const SUITS_GOOD = {
  'zh-CN': [
    'éæä¸æ®µéå¹´ä»£ç ?, 'åä¸ç¯ææ¯ç¬è®?, 'è®¤çåä¸æ¬?Code Review', 'Pair programming ä¸å°æ¶',
    'æä¸ä¸?draft PR', 'å³é­éç¥ä¸æ³¨ 90 åé', 'ç¨ä¾¿ç­¾çæ¸éæ±?, 'é¨ç½²ä¸æ¬¡å°æµè¯ç¯å¢',
    'è®¤çè¡¥ååæµè¯?, 'æä¸ä¸?TODO æ³¨éæ¸æ', 'è¯·åäºåä¸æ¯åå?, 'æ©ä¸ç¹ä¸ç­ï¼æ£æ­¥åå®¶',
    'ç»åéèµ·ä¸ªå¥½å¬çåå­', 'æ´æ°ä¾èµå°çæ?, 'éè¯»ä¸ä»½å¼æºé¡¹ç?README',
    'æèå­éçèå¾ç»å°ç½æ¿ä¸', 'ä¸ºææ®µä»£ç å ä¸æ®µä¸­ææ³¨é?, 'æ¸ç©ºä¸æ¬¡æ¡é¢æä»¶å¤¹',
    'åé¡¾ä¸å¨çå¾åï¼å æä¸¤æ¡', 'æä¸ä¸ªè?issue å³æ', 'åä¸æ®µéææµè¯?,
    'æä¸ä¸ªé¿å½æ°ææä¸¤ä¸ª', 'ç»é¡¹ç®å ä¸è¡?logging', 'ä¸»å¨åæ­¥ä¸æ¬¡è¿å±?,
    'è¯·æä¸ä¸ªä¸çæé¢åçåäº?, 'ä¸ºæ°äººåä¸ä»?å¦ä½ä¸æ"', 'æä¸ä¸?TODO è½¬æ issue',
    'å°è¯ä¸ä¸ªæ°çå¿«æ·é®', 'æä¸æ®?if-else æ¹ææ¥è¡¨', 'æä¸ä¸ªé­æ³æ°å­ææå¸¸é?,
    'ç¨çº¸ç¬æèååé', 'å°è¯ä¸ç§æ°çä¼æ¯èå¥?, 'å?commit message éå"ä¸ºä»ä¹?',
    'ååºä¸ä¸ªæç½®ç PR comment', 'ä¸»å¨ 1:1 ä¸ä½åäº?, 'ä¸ºä»å¤©å®ä¸ä¸ªæéè¦çç®æ ?,
    'å³æä¸¤ä¸ªé¿æä¸ççç¾¤', 'ä¸ºå¨æ¥åå¤ä¸æ®µäº®ç?, 'ææ··ä¹±ç imports æå¥½',
    'ä¸ºä¸ä¸ªè¾¹çæ¡ä»¶å ä¸ä¸ªæµè¯?, 'æ½ä¸æ®µæ¶é´å½»åºå®éå°æè?, 'æè°¢ä¸ä¸ªå¸®è¿ä½ çäºº',
  ],
  'zh-TW': [
    'éæ§ä¸æ®µé³å¹´ä»£ç¢?, 'å¯«ä¸ç¯æè¡ç­è¨?, 'èªçåä¸æ¬?Code Review', 'Pair programming ä¸å°æ',
    'æä¸å?draft PR', 'éééç¥å°æ³¨ 90 åé', 'ç¨ä¾¿ç±¤çæ¸éæ±?, 'é¨ç½²ä¸æ¬¡å°æ¸¬è©¦ç°å¢',
    'èªçè£å®åæ¸¬è©?, 'æä¸å?TODO è¨»éæ¸æ', 'è«åäºåä¸æ¯åå?, 'æ©ä¸é»ä¸ç­ï¼æ£æ­¥åå®¶',
    'çµ¦è®éèµ·åå¥½è½çåå­', 'æ´æ°ä¾è³´å°çæ?, 'é±è®ä¸ä»½éæºé ç?README',
    'æè¦å­è£¡çèåç«å°ç½æ¿ä¸', 'çºææ®µä»£ç¢¼å ä¸æ®µä¸­æè¨»é?, 'æ¸ç©ºä¸æ¬¡æ¡é¢æä»¶å¤¾',
    'åé¡§ä¸é±çå¾è¾¦ï¼åªæå©æ¢?, 'æä¸åè?issue éæ', 'å¯«ä¸æ®µéææ¸¬è©?,
    'æä¸åé·å½æ¸ææå©å?, 'çµ¦é ç®å ä¸è¡?logging', 'ä¸»ååæ­¥ä¸æ¬¡é²å±',
    'è«æä¸åä¸çæé åçåäº?, 'çºæ°äººå¯«ä¸ä»?å¦ä½ä¸æ"', 'æä¸å?TODO è½æ issue',
    'åè©¦ä¸åæ°çå¿«æ·éµ', 'æä¸æ®?if-else æ¹ææ¥è¡¨', 'æä¸åé­æ³æ¸å­ææå¸¸é?,
    'ç¨ç´ç­æèååé', 'åè©¦ä¸ç¨®æ°çä¼æ¯ç¯å¥?, 'å?commit message è£¡å¯«"çºä»éº?',
    'åæä¸åæ±ç½®ç PR comment', 'ä¸»å 1:1 ä¸ä½åäº?, 'çºä»å¤©å®ä¸åæéè¦çç®æ¨?,
    'éæå©åé·æä¸ççç¾?, 'çºé±å ±æºåä¸æ®µäº®é»?, 'ææ··äºç imports æå¥½',
    'çºä¸åéçæ¢ä»¶å ä¸åæ¸¬è©?, 'æ½ä¸æ®µæéå¾¹åºå®éå°æè?, 'æè¬ä¸åå¹«éä½ çäºº',
  ],

  'en-US': [
    'Refactor an old piece of code', 'Write a tech note', 'Do a real code review', 'Pair-program for an hour',
    'Open a draft PR', 'Mute notifications for 90 minutes', 'Lay out the requirements on sticky notes', 'Deploy once to staging',
    'Backfill some unit tests', 'Resolve one TODO comment', 'Buy a teammate coffee', 'Leave a bit early and walk home',
    'Pick a beautiful variable name', 'Bump a minor dependency', 'Read an open-source README',
    'Move the sketch in your head onto a whiteboard', 'Add a doc comment to a tricky block', 'Clean your desktop folder',
    'Drop two items from last week\'s todos', 'Close an old issue', 'Write one integration test',
    'Split a long function into two', 'Add one logging line to the project', 'Sync your progress proactively',
    'Ask an expert in an unfamiliar area', 'Write a "getting started" for newcomers', 'Turn a TODO into an issue',
    'Try a new keyboard shortcut', 'Replace an if-else chain with a lookup', 'Hoist a magic number into a constant',
    'Think with paper and pen for ten minutes', 'Try a new rest rhythm', 'Write the "why" in your commit message',
    'Reply to a stalled PR comment', 'Schedule a 1:1 with a teammate', 'Pick the most important goal of the day',
    'Mute two long-ignored chat rooms', 'Prep one highlight for the weekly report', 'Tidy up messy imports',
    'Add a test for an edge case', 'Take a stretch of true quiet thought', 'Thank someone who helped you',
  ],
};

const SUITS_BAD = {
  'zh-CN': [
    'å¨äºåæåå¸å°çäº?, 'ç´æ¥æ?main åæ¯', 'git push --force', 'è·³è¿æµè¯å°±åå¹?,
    'rm -rf ä¸çè·¯å¾', 'å¨æ²¡å¤ä»½æ¶æ¹æ°æ®åº?, 'npm install -g ä¸ççæ¬', 'å³æ CI éç¥',
    'å¨æç»ªæ¿å¨æ¶åå¤è¯è®º', 'æ?try { ... } catch {} çå¨ PR é?, 'ç¬å¤è°ä¸ä¸ªä¸è¡å°±è½æ¹ç?bug',
    'å¨æ²¡çæ¸éæ±æ¶å°±å¨æ?, 'ä¸ºäºèµ¶è¿åº¦è·³è¿?code review', 'åæ¶å¼åä¸ªåæ¯',
    'å?PR éå¤¹å¸¦ä¸ç¸å³çæ¹å?, 'å¨é¥¿èå­æ¶åæ¶æå³å®', 'åæ¨åçº¿ä¸åæ?,
    'å?review éåªè¯?LGTM"ä¸è§£é?, 'ä¸ºä¸ä¸ªç»èäºè®ºè¶è¿?30 åé', 'æ?hotfix ç´æ¥åå° main',
    'æ?ä»¥ååè¯´"åè¿æ³¨é', 'æ?print è°è¯å½ä½æ¥å¿', 'å¨ä¸çæçä»£ç éç²ç®å?try-catch',
    'ä¸è¾¹å¼ä¼ä¸è¾¹åå³é®ä»£ç ', 'åæ¶æ¿è¯ºä¸ä»¶äºé½ç»åä¸å¤?, 'å¨æ²¡ååç¡ç æ¶ä¸çº?,
    'åå¤å·æ° CI å½ä½ debug', 'å¨æç»ªä½è°·æ¶åèä¸å³å®?, 'å¨æ²¡ç?docs æ¶å°±éåå®?,
    'æ?review å½ä½"ææ¯ç?',
  ],
  'zh-TW': [
    'é±äºåæç¼ä½å°çç?, 'ç´æ¥æ?main åæ¯', 'git push --force', 'è·³éæ¸¬è©¦å°±åä½?,
    'rm -rf ä¸çè·¯å¾', 'å¨æ²åä»½ææ¹æ¸æåº?, 'npm install -g ä¸ççæ¬', 'éæ CI éç¥',
    'å¨æç·æ¿åæåè¦è©è«', 'æ?try { ... } catch {} çå¨ PR è£?, 'ç¬å¤èª¿ä¸åä¸è¡å°±è½æ¹ç?bug',
    'å¨æ²çæ¸éæ±æå°±åæ?, 'çºäºè¶é²åº¦è·³é code review', 'åæéåååæ?,
    'å?PR è£¡å¤¾å¸¶ä¸ç¸éçæ¹å?, 'å¨é¤èå­æåæ¶æ§æ±ºå®', 'åæ¨ç¼ç·ä¸è®æ?,
    'å?review è£¡åªèª?LGTM"ä¸è§£é?, 'çºä¸åç´°ç¯ç­è«è¶é 30 åé', 'æ?hotfix ç´æ¥åå° main',
    'æ?ä»¥å¾åèªª"å¯«é²è¨»é?, 'æ?print èª¿è©¦ç¶ä½æ¥èª', 'å¨ä¸çæçä»£ç¢¼è£¡ç²ç®å?try-catch',
    'ä¸ééæä¸éå¯«ééµä»£ç¢¼', 'åææ¿è«¾ä¸ä»¶äºé½çµ¦åä¸å¤?, 'å¨æ²ååç¡ç æä¸ç·?,
    'åè¦å·æ° CI ç¶ä½ debug', 'å¨æç·ä½è°·æåè·æ¥­æ±ºå®?, 'å¨æ²ç?docs æå°±éå¯«å®?,
    'æ?review ç¶ä½"ææ¯ç?',
  ],

  'en-US': [
    'Ship to production on a Friday evening', 'Push straight to main', 'git push --force', 'Merge without running tests',
    'rm -rf without checking the path', 'Touch the database without a backup', 'npm install -g without checking the version', 'Mute CI notifications',
    'Reply to a heated comment while heated', 'Leave try { ... } catch {} in the PR', 'Stay up all night for a one-line bug',
    'Start coding before reading the spec', 'Skip code review to "ship faster"', 'Open ten branches at once',
    'Sneak unrelated changes into a PR', 'Make architecture decisions while hungry', 'Push a production change at midnight',
    'Just say "LGTM" without explaining', 'Argue 30+ minutes over one detail', 'Merge a hotfix straight into main',
    'Write "later" in a comment', 'Use print statements as your logs', 'Wrap unknown code in blind try-catch',
    'Write critical code during a meeting', 'Promise three things due the same day', 'Deploy on too little sleep',
    'Re-trigger CI as a debugging strategy', 'Make career decisions during a low mood', 'Rewrite something before reading its docs',
    'Treat code review as nitpicking',
  ],
};

const COLORS = {
  'zh-CN': [
    { name: 'éé', hex: '#4f46e5' }, { name: 'ç«ç', hex: '#f472b6' }, { name: 'æ¹è', hex: '#06b6d4' },
    { name: 'æ£®ç»¿', hex: '#10b981' }, { name: 'æ©é', hex: '#f59e0b' }, { name: 'é¾ç´«', hex: '#a78bfa' },
    { name: 'ç çº¢', hex: '#ef4444' }, { name: 'éªç½', hex: '#f5f5f7' }, { name: 'ç­é»', hex: '#1f2937' },
    { name: 'è¶è¤', hex: '#92400e' }, { name: 'éç·', hex: '#5eead4' }, { name: 'æªé¦?, hex: '#c2956a' },
    { name: 'é»è', hex: '#3730a3' }, { name: 'é¶ç°', hex: '#94a3b8' }, { name: 'èç»¿', hex: '#65a30d' },
    { name: 'æ¢çº¢', hex: '#be185d' },
  ],
  'zh-TW': [
    { name: 'éé', hex: '#4f46e5' }, { name: 'ç«ç', hex: '#f472b6' }, { name: 'æ¹è', hex: '#06b6d4' },
    { name: 'æ£®ç¶ ', hex: '#10b981' }, { name: 'æ©é', hex: '#f59e0b' }, { name: 'é§ç´«', hex: '#a78bfa' },
    { name: 'ç£ç´', hex: '#ef4444' }, { name: 'éªç½', hex: '#f5f5f7' }, { name: 'ç­é»', hex: '#1f2937' },
    { name: 'è¶è¤', hex: '#92400e' }, { name: 'éç·', hex: '#5eead4' }, { name: 'æªé¦?, hex: '#c2956a' },
    { name: 'é»è', hex: '#3730a3' }, { name: 'éç?, hex: '#94a3b8' }, { name: 'èç¶ ', hex: '#65a30d' },
    { name: 'æ¢ç´', hex: '#be185d' },
  ],

  'en-US': [
    { name: 'Indigo', hex: '#4f46e5' }, { name: 'Rose Amber', hex: '#f472b6' }, { name: 'Lake Blue', hex: '#06b6d4' },
    { name: 'Forest Green', hex: '#10b981' }, { name: 'Amber Gold', hex: '#f59e0b' }, { name: 'Misty Violet', hex: '#a78bfa' },
    { name: 'Brick Red', hex: '#ef4444' }, { name: 'Snow White', hex: '#f5f5f7' }, { name: 'Charcoal', hex: '#1f2937' },
    { name: 'Tea Brown', hex: '#92400e' }, { name: 'Celadon', hex: '#5eead4' }, { name: 'Sandalwood', hex: '#c2956a' },
    { name: 'Slate Blue', hex: '#3730a3' }, { name: 'Silver Gray', hex: '#94a3b8' }, { name: 'Moss Green', hex: '#65a30d' },
    { name: 'Plum Red', hex: '#be185d' },
  ],
};

const HOURS = {
  'zh-CN': [
    'æ¸æ¨ 07:00 â?08:30', 'ä¸å 09:30 â?11:00', 'ä¸å 10:30 â?12:00',
    'æ­£å 12:00 â?13:00', 'ä¸å 14:00 â?15:30', 'ä¸å 15:30 â?17:00',
    'é»æ 17:30 â?19:00', 'å¤æ 20:00 â?21:30', 'å¤æ 21:00 â?22:30',
    'æ·±å¤ 22:00 â?23:30', 'æ·±å¤ 23:00 â?00:30', 'åæ¨ 05:30 â?07:00',
  ],
  'zh-TW': [
    'æ¸æ¨ 07:00 â?08:30', 'ä¸å 09:30 â?11:00', 'ä¸å 10:30 â?12:00',
    'æ­£å 12:00 â?13:00', 'ä¸å 14:00 â?15:30', 'ä¸å 15:30 â?17:00',
    'é»æ 17:30 â?19:00', 'å¤æ 20:00 â?21:30', 'å¤æ 21:00 â?22:30',
    'æ·±å¤ 22:00 â?23:30', 'æ·±å¤ 23:00 â?00:30', 'åæ¨ 05:30 â?07:00',
  ],

  'en-US': [
    'Early morning 07:00 â?08:30', 'Morning 09:30 â?11:00', 'Late morning 10:30 â?12:00',
    'Midday 12:00 â?13:00', 'Afternoon 14:00 â?15:30', 'Afternoon 15:30 â?17:00',
    'Dusk 17:30 â?19:00', 'Evening 20:00 â?21:30', 'Evening 21:00 â?22:30',
    'Late night 22:00 â?23:30', 'Late night 23:00 â?00:30', 'Pre-dawn 05:30 â?07:00',
  ],
};

const MANTRAS = {
  'zh-CN': [
    'It compiles. Ship it.',
    'Make it work, make it right, make it fast.',
    'Done is better than perfect.',
    'Premature optimization is the root of all evil.',
    'Read the source, Luke.',
    'Stay hungry, stay foolish.',
    'Talk is cheap, show me the code.',
    'æå¥½çä»£ç ï¼æ¯ä¸å¿åçä»£ç ã?,
    'ä¸æ¬¡åªè§£å³ä¸ä¸ªé®é¢ã?,
    'è½è·èµ·æ¥ï¼å°±åè·èµ·æ¥ã?,
    'ç¸ä¿¡ä½ çä¸ä¸ä¸?git commitã?,
    'ä»å¤©çæï¼ä¸è¯å¤è¿å»çæã?,
    'ç®åä¼äºå¤æï¼æç¡®ä¼äºèªæã?,
    'å®å¯åä¸¤éï¼ä¹å«éæ½è±¡ä¸æ¬¡ã?,
    'åç»äººè¯»çä»£ç ï¼é¡ºä¾¿è½å¨æºå¨ä¸è·ã?,
    'ä»æ¥å°åä¸äºï¼æå¤©å¤èµ°ä¸äºã?,
    'èµ°å¾æ¢ä¸ç¹ï¼ä½å«åä¸æ¥ã?,
    'åè®¸å®åä¸éå°å·¥ä½ï¼åä¼éå°å·¥ä½ã?,
    'åå­åå¾å¥½ï¼bug å°±å°ä¸åã?,
    'ä¸å¶å®ç¾å°åä¸ä»¶äºï¼ä¸å¦åå®ä¸ä»¶äºã?,
    'å«ä¿¡"ä»¥åä¼éå?ï¼ä½åè®¸"ç°å¨è½ç¨"ã?,
    'åè®¸èªå·±ä»å¤©åªåä¸ä»¶å¥½äºã?,
    'æçä½ çåè®¾ï¼ä¸è¦æçä½ çä»·å¼ã?,
    'ä»å¤©æå¨ä½ çï¼æªå¿è½æå¨åå¹´åçä½ ã?,
    'ä¸åä»£ç é½æ¯åºï¼ä»å¤©è¿ä¸ç¹ã?,
    'åæåé¦ï¼åæå®ç¾ã?,
    'Done > Perfect > Started > Nothing.',
    'ç¸ä¿¡èå¥ï¼ç¸ä¿¡å¤å©ã?,
  ],
  'zh-TW': [
    'It compiles. Ship it.',
    'Make it work, make it right, make it fast.',
    'Done is better than perfect.',
    'Premature optimization is the root of all evil.',
    'Read the source, Luke.',
    'Stay hungry, stay foolish.',
    'Talk is cheap, show me the code.',
    'æå¥½çä»£ç¢¼ï¼æ¯ä¸å¿å¯«çä»£ç¢¼ã?,
    'ä¸æ¬¡åªè§£æ±ºä¸ååé¡ã?,
    'è½è·èµ·ä¾ï¼å°±åè·èµ·ä¾ã?,
    'ç¸ä¿¡ä½ çä¸ä¸å?git commitã?,
    'ä»å¤©çæï¼ä¸è©å¤éå»çæã?,
    'ç°¡å®åªæ¼è¤éï¼æç¢ºåªæ¼è°æã?,
    'å¯§å¯å¯«å©éï¼ä¹å¥é¯æ½è±¡ä¸æ¬¡ã?,
    'å¯«çµ¦äººè®çä»£ç¢¼ï¼é ä¾¿è½å¨æ©å¨ä¸è·ã?,
    'ä»æ¥å°åä¸äºï¼æå¤©å¤èµ°ä¸äºã?,
    'èµ°å¾æ¢ä¸é»ï¼ä½å¥åä¸ä¾ã?,
    'åè¨±å®åééå°å·¥ä½ï¼ååªéå°å·¥ä½ã?,
    'åå­åå¾å¥½ï¼bug å°±å°ä¸åã?,
    'èå¶å®ç¾å°åä¸ä»¶äºï¼ä¸å¦åå®ä¸ä»¶äºã?,
    'å¥ä¿¡"ä»¥å¾æéå¯?ï¼ä½åè¨±"ç¾å¨è½ç¨"ã?,
    'åè¨±èªå·±ä»å¤©åªåä¸ä»¶å¥½äºã?,
    'æ·çä½ çåè¨­ï¼ä¸è¦æ·çä½ çå¹å¼ã?,
    'ä»å¤©æåä½ çï¼æªå¿è½æååå¹´å¾çä½ ã?,
    'ä¸åä»£ç¢¼é½æ¯åµï¼ä»å¤©éä¸é»ã?,
    'åæåé¥ï¼åæå®ç¾ã?,
    'Done > Perfect > Started > Nothing.',
    'ç¸ä¿¡ç¯å¥ï¼ç¸ä¿¡è¤å©ã?,
  ],

  'en-US': [
    'It compiles. Ship it.',
    'Make it work, make it right, make it fast.',
    'Done is better than perfect.',
    'Premature optimization is the root of all evil.',
    'Read the source, Luke.',
    'Stay hungry, stay foolish.',
    'Talk is cheap, show me the code.',
    'The best code is the code you don\'t have to write.',
    'Solve one problem at a time.',
    'Get it running first; then get it right.',
    'Trust your next git commit.',
    'Today\'s me does not judge yesterday\'s me.',
    'Simple beats complex; explicit beats clever.',
    'Better write it twice than abstract it wrong once.',
    'Write code humans read; the machine runs it as a bonus.',
    'Do a little less today; walk a little further tomorrow.',
    'Walk slowly, but don\'t stop.',
    'Let it work ugly first; make it elegant later.',
    'A great name halves the bugs.',
    'Finishing one thing beats perfecting it.',
    'Don\'t bet on "I\'ll rewrite later" â?bet on "this works now".',
    'Allow yourself one good thing today.',
    'Question your assumptions, never your worth.',
    'What moves you today may not move you in six months.',
    'All code is debt â?pay a little today.',
    'Feedback first, perfection later.',
    'Done > Perfect > Started > Nothing.',
    'Trust rhythm; trust compounding.',
  ],
};

const INSIGHTS = {
  'zh-CN': [
    'ä»æ¥çæ³¨æåæ¯æ¶é´æ´ç¨ç¼ºï¼è¯·ä¼ååéã?,
    'ä¸å¶è¿½æ±"ä»å¤©åå®ä»ä¹?ï¼ä¸å¦ç¡®è®?ä»å¤©å¾åªèµ°"ã?,
    'ç¢°å°ç¬¬ä¸æ¬¡çéº»ç¦ï¼å°±è¯¥æå®å°è£æå½æ°ã?,
    'ä¸å¶ä¿®åä¸ªå° bugï¼ä¸å¦æéä¸ä¸ªæ ¹å ã?,
    'ä¸ä¸ªå¹²åçæ¡é¢ï¼å¸¸å¸¸å¸¦æ¥ä¸ä¸ªå¹²åçæè·¯ã?,
    'æ?ææè§?æ¢æ"æçå°äº"ã?,
    'å½æ¹æ¡å¤ªå¤æ¶ï¼è¯´æé®é¢æ²¡é®å¯¹ã?,
    'è®©å«äººå°çä¸æ¬¡ï¼å¢éå°±å¿«ä¸åã?,
    'é«é¢å°åæ­¥ï¼èè¿å¶å°å¤§å¯¹é½ã?,
    'å½ä»£ç é¾åï¼å¾å¾æ¯è®¾è®¡å¨æ±æã?,
    'ä»å¤©çåé¦å¾ªç¯è¶ç­ï¼æå¤©çä¸ç¡®å®è¶å°ã?,
    'å¦æä½ æ³å ä¸ä¸ªç¹ä¾ï¼åæ³æ³æ¯ä¸æ¯æ¨¡åéäºã?,
    'å«åªé?è½ä¸è½å"ï¼ä¹é?è¯¥ä¸è¯¥å"ã?,
    'æ¯ä¸æ¬?pushï¼é½æ¯ç»æªæ¥çèªå·±åä¿¡ã?,
    'å°å³å®é ä¹ æ¯ï¼å¤§å³å®é ç¡ä¸è§ã?,
    'ä¸ä¸ªç¨³å®çå·¥å·é¾ï¼èè¿åä¸ªç«æã?,
    'æä¼è®®åå°ï¼æææ¡£åå¥½ã?,
    'ä»æ¥å®ç 10% çä½åç»æå¤ã?,
    'å½å´è¶£æ¥æ²é¨ï¼è¯·å®è¿æ¥å 10 åéã?,
    'è§å¯ä¸æ¬¡èªå·±çæå»¶ï¼ä¸è¯å¤ï¼åªè®°å½ã?,
    'æä¸æ®µéå¤æä½èæ¬åï¼æªæ¥ä½ ä¼ç¬åºå£°ã?,
    'è¯¥åæµè¯æ¶åæµè¯ï¼è¯¥ç¡è§æ¶ç¡è§ã?,
    'ä¸æ³¨æ¯ç§ç»ä¹ ï¼ä»å¤©åæ¯ä¸ä¸?setã?,
    'å½ä½ æ³æ¾å¼ï¼åå»åä¸æ¯æ°´åè¯´ã?,
    'ä»å¤©éå°çæ¯ä¸ä¸?stack traceï¼é½æ¯åè´¹çè¯¾ã?,
    'ä¸çæçé¢åï¼åå¤è¿°ä¸éåå¨æã?,
    'å½ä»£ç è¯å®¡è®©ä½ ä¸èæï¼å¤åå»ä¸­äºçé®é¢ã?,
    'æ?é?ææ"åååªä¸æ­?ï¼é¾å°±å¼å§æ¶è§£ã?,
    'åè®¸èªå·±ä»å¤©åªäº¤ä»?60 åï¼æå¤©åè¿­ä»£ã?,
    'ç¸ä¿¡å¤å©ï¼ä½å«å¿äºä»å¤©å°±æ¯å©æ¯ã?,
  ],
  'zh-TW': [
    'ä»æ¥çæ³¨æåæ¯æéæ´ç¨ç¼ºï¼è«åªååéã?,
    'èå¶è¿½æ±"ä»å¤©åå®ä»éº?ï¼ä¸å¦ç¢ºèª?ä»å¤©å¾åªèµ°"ã?,
    'ç¢°å°ç¬¬ä¸æ¬¡çéº»ç©ï¼å°±è©²æå®å°è£æå½æ¸ã?,
    'èå¶ä¿®ååå° bugï¼ä¸å¦æéä¸åæ ¹å ã?,
    'ä¸åä¹¾æ·¨çæ¡é¢ï¼å¸¸å¸¸å¸¶ä¾ä¸åä¹¾æ·¨çæè·¯ã?,
    'æ?ææè¦?ææ"æçå°äº"ã?,
    'ç¶æ¹æ¡å¤ªå¤æï¼èªªæåé¡æ²åå°ã?,
    'è®å¥äººå°çä¸æ¬¡ï¼åéå°±å¿«ä¸åã?,
    'é«é »å°åæ­¥ï¼åéå¶ç¾å¤§å°é½ã?,
    'ç¶ä»£ç¢¼é£å¯«ï¼å¾å¾æ¯è¨­è¨å¨æ±æã?,
    'ä»å¤©çåé¥å¾ªç°è¶ç­ï¼æå¤©çä¸ç¢ºå®è¶å°ã?,
    'å¦æä½ æ³å ä¸åç¹ä¾ï¼åæ³æ³æ¯ä¸æ¯æ¨¡åé¯äºã?,
    'å¥é»å?è½ä¸è½å"ï¼ä¹å?è©²ä¸è©²å"ã?,
    'æ¯ä¸æ¬?pushï¼é½æ¯çµ¦æªä¾çèªå·±å¯«ä¿¡ã?,
    'å°æ±ºå®é ç¿æ£ï¼å¤§æ±ºå®é ç¡ä¸è¦ºã?,
    'ä¸åç©©å®çå·¥å·éï¼åéååç«æã?,
    'ææè­°è®å°ï¼æææªè®å¥½ã?,
    'ä»æ¥å®ç 10% çé¤åçµ¦æå¤ã?,
    'ç¶èè¶£ä¾æ²éï¼è«å®é²ä¾å?10 åéã?,
    'è§å¯ä¸æ¬¡èªå·±çæå»¶ï¼ä¸è©å¤ï¼åªè¨éã?,
    'æä¸æ®µéè¤æä½è³æ¬åï¼æªä¾ä½ æç¬åºè²ã?,
    'è©²å¯«æ¸¬è©¦æå¯«æ¸¬è©¦ï¼è©²ç¡è¦ºæç¡è¦ºã?,
    'å°æ³¨æ¯ç¨®ç·´ç¿ï¼ä»å¤©åæ¯ä¸å?setã?,
    'ç¶ä½ æ³æ¾æ£ï¼åå»åä¸æ¯æ°´åèªªã?,
    'ä»å¤©éå°çæ¯ä¸å?stack traceï¼é½æ¯åè²»çèª²ã?,
    'ä¸çæçé åï¼åè¤è¿°ä¸éååæã?,
    'ç¶ä»£ç¢¼è©å¯©è®ä½ ä¸èæï¼å¤åæä¸­äºçåé¡ã?,
    'æ?é?ææ"åååªä¸æ­?ï¼é£å°±éå§æ¶è§£ã?,
    'åè¨±èªå·±ä»å¤©åªäº¤ä»?60 åï¼æå¤©åè¿­ä»£ã?,
    'ç¸ä¿¡è¤å©ï¼ä½å¥å¿äºä»å¤©å°±æ¯å©æ¯ã?,
  ],

  'en-US': [
    'Today, attention is scarcer than time â?allocate it first.',
    'Instead of "what to finish today", decide "which way to head today".',
    'When trouble hits a third time, wrap it in a function.',
    'Better to dig through one root cause than patch ten symptoms.',
    'A clean desktop often brings a clean train of thought.',
    'Replace "I feel" with "I saw".',
    'Too many solutions usually means the wrong question.',
    'When others have to guess less, the team moves twice as fast.',
    'High-frequency small syncs beat occasional big alignments.',
    'When code is hard to write, design is asking for help.',
    'Shorter feedback loop today; less uncertainty tomorrow.',
    'If you want to add a special case, ask if the model is wrong.',
    'Don\'t just ask "can we do it" â?also ask "should we".',
    'Every push is a letter to your future self.',
    'Small decisions ride habits; big decisions ride a good sleep.',
    'One stable toolchain beats ten flashy tricks.',
    'Make meetings smaller; make docs better.',
    'Reserve 10% of today\'s capacity for surprises.',
    'When curiosity knocks, let it sit for ten minutes.',
    'Observe your procrastination once â?no judgment, just notes.',
    'Script a repetitive task; future-you will laugh out loud.',
    'Write tests when you should; sleep when you should.',
    'Focus is a practice; today is another set.',
    'When you want to give up, pour a glass of water first.',
    'Every stack trace today is a free lesson.',
    'In unfamiliar territory, paraphrase first, code second.',
    'When code review makes you uncomfortable, it usually struck a real issue.',
    'Break "hard" into "what\'s the first step" â?and hard starts dissolving.',
    'Allow yourself a 60-point delivery today; iterate tomorrow.',
    'Trust compounding â?but remember: today is the interest payment.',
  ],
};

const UI_I18N = {
  'zh-CN': {
    title: 'æ¯æ¥å å',
    spreadAria: 'ä»æ¥çéµ',
    fortuneMatrix: 'è¿å¿ç©éµ',
    todayGood: 'ä»æ¥å®?,
    todayBad: 'ä»æ¥å¿?,
    omenTitle: 'æºç¼æç¤º',
    luckyColor: 'å¹¸è¿è?,
    luckyNumber: 'å¹¸è¿æ°å­',
    luckyHour: 'æ¨èæ¶æ®µ',
    mantra: 'åè¯­',
    copyText: 'å¤å¶è¿å¿ææ¬',
    footerHint: 'æ¿ä½ ä»æ¥çä»£ç æ  bugï¼commit æ»è½éè¿ reviewã?,
    greetingFresh: 'åç¥',
    greetingDrawn: 'ä»æ¥å¦è±¡å·²ç«',
    subtitleFresh: 'è½»è§¦ä¸å¼ çï¼æ­å¼ä»æ¥å¦è±¡',
    subtitleDrawn: 'æ½ä¸å¼ çä»¥éæ¸?,
    tipFresh: 'æ¯æ¥å¦è±¡ä¸æ¦æ¾ç°ä¾¿å·²æ³¨å®?Â· ç¿æ¥ 00:00 çæ°',
    tipDrawn: 'å¦è±¡å·²æ³¨å®?Â· ä»ªå¼ä»ä¾åå³',
    cardAriaLabel: (i) => `ç¬?${i} å¼ ç`,
    todayInsightLabel: 'â?ä»æ¥æ´å¯ â?,
    fortuneOverall: 'ç»¼å', fortuneWork: 'å·¥ä½', fortuneInspire: 'çµæ', fortuneWealth: 'è´¢è¿',
    dateFormat: ({ y, m, d }) => `${y} å¹?${m} æ?${d} æ¥`,
    shareCardLine: (name, keyword) => `ã?{name}ã?${keyword}`,
    shareInsight: (text) => `ä»æ¥æ´å¯ï¼?{text}`,
    shareGood: (list) => `ä»æ¥å®ï¼${list.join('ã?)}`,
    shareBad: (list) => `ä»æ¥å¿ï¼${list.join('ã?)}`,
    shareLucky: (color, n, hour) => `å¹¸è¿è²ï¼${color}ãå¹¸è¿æ°å­ï¼?{n}ãæ¨èæ¶æ®µï¼?{hour}`,
    shareMantra: (text) => `åè¯­ï¼?{text}`,
    toastCopied: 'å·²å¤å¶å°åªè´´æ?,
    toastCopyFailed: 'å¤å¶å¤±è´¥',
  },
  'zh-TW': {
    title: 'æ¯æ¥ä½å',
    spreadAria: 'ä»æ¥çé£',
    fortuneMatrix: 'éå¢ç©é£',
    todayGood: 'ä»æ¥å®?,
    todayBad: 'ä»æ¥å¿?,
    omenTitle: 'æ©ç·£æç¤º',
    luckyColor: 'å¹¸éè?,
    luckyNumber: 'å¹¸éæ¸å­',
    luckyHour: 'æ¨è¦ææ®µ',
    mantra: 'åèª',
    copyText: 'è¤è£½éå¢ææ¬',
    footerHint: 'é¡ä½ ä»æ¥çä»£ç¢¼ç¡ bugï¼commit ç¸½è½éé reviewã?,
    greetingFresh: 'åç¥',
    greetingDrawn: 'ä»æ¥å¦è±¡å·²ç«',
    subtitleFresh: 'è¼è§¸ä¸å¼µçï¼æ­éä»æ¥å¦è±?,
    subtitleDrawn: 'æ½ä¸å¼µçä»¥éæº?,
    tipFresh: 'æ¯æ¥å¦è±¡ä¸æ¦é¡¯ç¾ä¾¿å·²è¨»å®?Â· ç¿æ¥ 00:00 ç¥æ°',
    tipDrawn: 'å¦è±¡å·²è¨»å®?Â· åå¼åä¾åå?,
    cardAriaLabel: (i) => `ç¬?${i} å¼µç`,
    todayInsightLabel: 'â?ä»æ¥æ´å¯ â?,
    fortuneOverall: 'ç¶å', fortuneWork: 'å·¥ä½', fortuneInspire: 'éæ', fortuneWealth: 'è²¡é',
    dateFormat: ({ y, m, d }) => `${y} å¹?${m} æ?${d} æ¥`,
    shareCardLine: (name, keyword) => `ã?{name}ã?${keyword}`,
    shareInsight: (text) => `ä»æ¥æ´å¯ï¼?{text}`,
    shareGood: (list) => `ä»æ¥å®ï¼${list.join('ã?)}`,
    shareBad: (list) => `ä»æ¥å¿ï¼${list.join('ã?)}`,
    shareLucky: (color, n, hour) => `å¹¸éè²ï¼${color}ãå¹¸éæ¸å­ï¼?{n}ãæ¨è¦ææ®µï¼?{hour}`,
    shareMantra: (text) => `åèªï¼?{text}`,
    toastCopied: 'å·²è¤è£½å°åªè²¼æ?,
    toastCopyFailed: 'è¤è£½å¤±æ',
  },

  'en-US': {
    title: 'Daily Divination',
    spreadAria: 'Today\'s spread',
    fortuneMatrix: 'Fortune matrix',
    todayGood: 'Do',
    todayBad: 'Don\'t',
    omenTitle: 'Lucky omens',
    luckyColor: 'Lucky color',
    luckyNumber: 'Lucky number',
    luckyHour: 'Best hours',
    mantra: 'Mantra',
    copyText: 'Copy reading',
    footerHint: 'May your code be bug-free and your commits always pass review.',
    greetingFresh: 'Center yourself',
    greetingDrawn: 'Today\'s reading is set',
    subtitleFresh: 'Tap a card to reveal today\'s fortune',
    subtitleDrawn: 'Draw any card to revisit',
    tipFresh: 'Today\'s fortune is fixed once revealed Â· refreshes at 00:00 tomorrow',
    tipDrawn: 'The reading is set Â· the ritual is for reflection',
    cardAriaLabel: (i) => `Card ${i}`,
    todayInsightLabel: 'â?Today\'s Insight â?,
    fortuneOverall: 'Overall', fortuneWork: 'Work', fortuneInspire: 'Inspiration', fortuneWealth: 'Wealth',
    dateFormat: ({ y, m, d }) => {
      const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
      return `${months[Number(m) - 1]} ${Number(d)}, ${y}`;
    },
    shareCardLine: (name, keyword) => `[${name}] ${keyword}`,
    shareInsight: (text) => `Insight: ${text}`,
    shareGood: (list) => `Do: ${list.join(', ')}`,
    shareBad: (list) => `Don't: ${list.join(', ')}`,
    shareLucky: (color, n, hour) => `Lucky color: ${color}   Lucky number: ${n}   Best hours: ${hour}`,
    shareMantra: (text) => `Mantra: ${text}`,
    toastCopied: 'Copied to clipboard',
    toastCopyFailed: 'Copy failed',
  },
};

function currentLocale() {
  return (window.app && window.app.locale) || 'en-US';
}
function ui(key) {
  const lang = currentLocale();
  const table = UI_I18N[lang] || UI_I18N['en-US'];
  return table[key];
}

function getCards() {
  const lang = currentLocale();
  const strings = CARD_STRINGS[lang] || CARD_STRINGS['en-US'];
  return strings.map((s, i) => ({ ...CARD_VISUALS[i], ...s }));
}

function getFortuneLabels() {
  return [
    { key: 'overall', label: ui('fortuneOverall') },
    { key: 'work',    label: ui('fortuneWork') },
    { key: 'inspire', label: ui('fortuneInspire') },
    { key: 'wealth',  label: ui('fortuneWealth') },
  ];
}

// ââ Random utilities (seeded) ââââââââââââââââââââââââ
function dateKey(d = new Date()) {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function hashSeed(s) {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function mulberry32(seed) {
  let t = seed >>> 0;
  return function () {
    t = (t + 0x6d2b79f5) >>> 0;
    let r = Math.imul(t ^ (t >>> 15), 1 | t);
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

function pickIdx(rand, len) {
  return Math.floor(rand() * len);
}

function pickIndices(rand, len, n) {
  // Sample `n` distinct indices in [0, len). Order matches the original
  // `pickN(rand, arr, n)` so localized arrays of equal length yield matching
  // selections across languages.
  const pool = [];
  for (let i = 0; i < len; i++) pool.push(i);
  const out = [];
  for (let i = 0; i < n && pool.length > 0; i++) {
    const idx = Math.floor(rand() * pool.length);
    out.push(pool.splice(idx, 1)[0]);
  }
  return out;
}

// ââ Fortune generation âââââââââââââââââââââââââââââââ
// `generateFortune` returns INDICES + raw stars. Localization happens at render
// time so changing language re-renders the same reading in another tongue.
function generateFortuneIndices(date) {
  const seed = hashSeed('northhing-divination-' + date);
  const rand = mulberry32(seed);

  const cardIdx = Math.floor(rand() * CARD_VISUALS.length);

  const stars = FORTUNE_KEY_IDS.map(() => {
    const r = rand();
    return r < 0.06 ? 1 : r < 0.2 ? 2 : r < 0.55 ? 3 : r < 0.85 ? 4 : 5;
  });

  // Quote index inside the chosen card. CARD_STRINGS for both locales must
  // expose the same number of quotes per card, which is the case here.
  const zhQuotes = CARD_STRINGS['zh-CN'][cardIdx].quotes;
  const quoteIdx = Math.floor(rand() * zhQuotes.length);

  const insightIdx = Math.floor(rand() * INSIGHTS['zh-CN'].length);
  const goodIndices = pickIndices(rand, SUITS_GOOD['zh-CN'].length, 3);
  const badIndices  = pickIndices(rand, SUITS_BAD['zh-CN'].length, 2);
  const colorIdx = Math.floor(rand() * COLORS['zh-CN'].length);
  const luckyNumber = 1 + Math.floor(rand() * 99);
  const hourIdx = Math.floor(rand() * HOURS['zh-CN'].length);
  const mantraIdx = Math.floor(rand() * MANTRAS['zh-CN'].length);

  return { cardIdx, stars, quoteIdx, insightIdx, goodIndices, badIndices, colorIdx, luckyNumber, hourIdx, mantraIdx };
}

function localizeFortune(indices) {
  const cards = getCards();
  const card = cards[indices.cardIdx];
  const lang = currentLocale();
  const insights = INSIGHTS[lang] || INSIGHTS['en-US'];
  const good = SUITS_GOOD[lang] || SUITS_GOOD['en-US'];
  const bad = SUITS_BAD[lang] || SUITS_BAD['en-US'];
  const colors = COLORS[lang] || COLORS['en-US'];
  const hours = HOURS[lang] || HOURS['en-US'];
  const mantras = MANTRAS[lang] || MANTRAS['en-US'];
  const fortunes = getFortuneLabels().map((f, i) => ({ ...f, stars: indices.stars[i] }));
  return {
    card,
    quote: card.quotes[indices.quoteIdx % card.quotes.length],
    insight: insights[indices.insightIdx % insights.length],
    fortunes,
    goods: indices.goodIndices.map((i) => good[i % good.length]),
    bads:  indices.badIndices.map((i) => bad[i % bad.length]),
    color: colors[indices.colorIdx % colors.length],
    luckyNumber: indices.luckyNumber,
    hour: hours[indices.hourIdx % hours.length],
    mantra: mantras[indices.mantraIdx % mantras.length],
  };
}

// ââ DOM ââââââââââââââââââââââââââââââââââââââââââââââ
const dom = {
  dateLabel: document.getElementById('date-label'),
  drawStage: document.getElementById('draw-stage'),
  resultStage: document.getElementById('result-stage'),
  cardSpread: document.getElementById('card-spread'),
  greeting: document.getElementById('greeting'),
  drawSubtitle: document.getElementById('draw-subtitle'),
  drawTip: document.getElementById('draw-tip'),
  cardFront: document.getElementById('card-front'),
  cardIndex: document.getElementById('card-index'),
  cardTag: document.getElementById('card-tag'),
  cardArt: document.getElementById('card-art'),
  cardName: document.getElementById('card-name'),
  cardKeyword: document.getElementById('card-keyword'),
  cardQuote: document.getElementById('card-quote'),
  cardInsight: document.getElementById('card-insight'),
  fortunes: document.getElementById('fortunes'),
  suitGood: document.getElementById('suit-good'),
  suitBad: document.getElementById('suit-bad'),
  luckyColorSwatch: document.getElementById('lucky-color-swatch'),
  luckyColorName: document.getElementById('lucky-color-name'),
  luckyNumber: document.getElementById('lucky-number'),
  luckyHour: document.getElementById('lucky-hour'),
  luckyMantra: document.getElementById('lucky-mantra'),
  btnShare: document.getElementById('btn-share'),
  toast: document.getElementById('toast'),
};

// We keep the deterministic *indices* (computed from the date) plus whether the
// reading was already drawn â?so a locale change can simply re-render in place.
let currentIndices = null;
let currentDate = null;
let currentDrawn = false;

function fmtDate(date) {
  const [y, m, d] = date.split('-');
  return ui('dateFormat')({ y, m: String(parseInt(m, 10)), d: String(parseInt(d, 10)) });
}

function applyStaticI18n() {
  document.documentElement.setAttribute('lang', currentLocale());
  document.querySelectorAll('[data-i18n]').forEach((node) => {
    const key = node.getAttribute('data-i18n');
    const attr = node.getAttribute('data-i18n-attr');
    const value = ui(key);
    if (typeof value !== 'string') return;
    if (attr) node.setAttribute(attr, value);
    else node.textContent = value;
  });
}

// ââ Card-back symbols (purely cosmetic; the actual fortune is fixed by date) ââ
const BACK_SYMBOLS = ['â?, 'â?, 'â?, 'â?, 'â?, 'â?, 'â?, 'â?, 'â?];

function applySceneTone(tone) {
  // Dye the entire scene (background, aurora, card, accents) with the day's
  // card tone so the room feels monochromatic â?no clash between purple bg
  // and a blue card. tone[0] is the bright accent, tone[1] is deep shadow.
  const root = document.querySelector('.div-app') || document.body;
  root.style.setProperty('--card-tone-1', tone[0]);
  root.style.setProperty('--card-tone-2', tone[1]);
  if (dom.cardFront) {
    dom.cardFront.style.setProperty('--card-tone-1', tone[0]);
    dom.cardFront.style.setProperty('--card-tone-2', tone[1]);
  }
  if (dom.resultStage) {
    dom.resultStage.style.setProperty('--card-tone-1', tone[0]);
    dom.resultStage.style.setProperty('--card-tone-2', tone[1]);
  }
}

async function init() {
  applyStaticI18n();
  const today = dateKey();
  currentDate = today;
  dom.dateLabel.textContent = fmtDate(today);

  let saved = null;
  try { saved = await app.storage.get('lastReading'); } catch (_e) { /* ignore */ }
  currentDrawn = !!(saved && saved.date === today);
  setupDraw(today, currentDrawn);

  if (window.app && typeof window.app.onLocaleChange === 'function') {
    window.app.onLocaleChange(() => {
      applyStaticI18n();
      if (currentDate) dom.dateLabel.textContent = fmtDate(currentDate);
      // If the user hasn't picked yet, refresh draw labels.
      if (!currentIndices) {
        setupDraw(currentDate, currentDrawn);
      } else {
        // Otherwise re-render the result card in the new language.
        paintResult(localizeFortune(currentIndices));
      }
    });
  }
}

function setupDraw(today, alreadyDrawn) {
  dom.drawStage.hidden = false;
  dom.resultStage.hidden = true;
  dom.resultStage.classList.remove('is-active');
  if (alreadyDrawn) {
    dom.greeting.textContent = ui('greetingDrawn');
    dom.drawSubtitle.textContent = ui('subtitleDrawn');
    dom.drawTip.textContent = ui('tipDrawn');
  } else {
    dom.greeting.textContent = ui('greetingFresh');
    dom.drawSubtitle.textContent = ui('subtitleFresh');
    dom.drawTip.textContent = ui('tipFresh');
  }

  dom.cardSpread.innerHTML = '';
  const seed = hashSeed('spread-' + today);
  const rand = mulberry32(seed);
  const symbols = BACK_SYMBOLS.slice();
  for (let i = symbols.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [symbols[i], symbols[j]] = [symbols[j], symbols[i]];
  }
  const fan = symbols.slice(0, 5);
  fan.forEach((sym, i) => {
    const angle = (i - 2) * 8;
    const lift = Math.abs(i - 2) * 14;
    const card = document.createElement('div');
    card.className = 'card-pick';
    card.style.setProperty('--rot', angle + 'deg');
    card.style.setProperty('--y', lift + 'px');
    card.style.setProperty('--enter-delay', (i * 90) + 'ms');
    card.style.zIndex = String(10 - Math.abs(i - 2));
    card.tabIndex = 0;
    card.setAttribute('role', 'button');
    card.setAttribute('aria-label', ui('cardAriaLabel')(i + 1));
    card.dataset.idx = String(i);
    card.innerHTML = `
      <div class="card-pick__pattern"></div>
      <div class="card-pick__inner">
        <div class="card-pick__symbol">${sym}</div>
      </div>
      <div class="card-pick__shine"></div>
    `;
    const handler = () => onPick(card, today, alreadyDrawn);
    card.addEventListener('click', handler);
    card.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handler(); }
    });
    dom.cardSpread.appendChild(card);
  });
}

let pickInFlight = false;
function spawnBurst(centerEl) {
  // Center the burst on the chosen card, fall back to viewport center.
  let x = window.innerWidth / 2;
  let y = window.innerHeight / 2;
  if (centerEl && centerEl.getBoundingClientRect) {
    const rect = centerEl.getBoundingClientRect();
    x = rect.left + rect.width / 2;
    y = rect.top + rect.height / 2;
  }
  const burst = document.createElement('div');
  burst.className = 'draw-burst';
  burst.style.left = x + 'px';
  burst.style.top = y + 'px';
  document.body.appendChild(burst);
  const veil = document.createElement('div');
  veil.className = 'draw-veil';
  document.body.appendChild(veil);
  setTimeout(() => { burst.remove(); veil.remove(); }, 1300);
}

function onPick(chosen, today, alreadyDrawn) {
  if (pickInFlight) return;
  pickInFlight = true;
  // Compute scatter directions for the discarded cards so they fly outward.
  const cards = Array.from(dom.cardSpread.children);
  const chosenIdx = cards.indexOf(chosen);
  for (let i = 0; i < cards.length; i++) {
    const card = cards[i];
    card.style.pointerEvents = 'none';
    card.tabIndex = -1;
    if (card !== chosen) {
      const dir = i - chosenIdx;
      const dx = dir * 160 + (dir < 0 ? -80 : 80);
      const rot = dir * 18;
      card.style.setProperty('--scatter-x', dx + 'px');
      card.style.setProperty('--scatter-rot', rot + 'deg');
      card.classList.add('is-discarded');
    }
  }
  chosen.classList.add('is-chosen');
  // Pre-compute the day's card so we can start the scene-tone transition
  // in lockstep with the burst+flip animation. CSS will animate `.div-app`
  // background over ~1.4s, so by the time the result is revealed the room
  // is already breathing the new card's color.
  const indices = generateFortuneIndices(today);
  const tone = CARD_VISUALS[indices.cardIdx].tone;
  // After the lift settles, trigger the flip-into-burst sequence.
  setTimeout(() => {
    spawnBurst(chosen);
    chosen.classList.add('is-flipping');
    applySceneTone(tone);
  }, 380);
  setTimeout(() => revealResult(today, alreadyDrawn), 1280);
}

function revealResult(today, alreadyDrawn) {
  currentIndices = generateFortuneIndices(today);
  const fortune = localizeFortune(currentIndices);
  paintResult(fortune);
  dom.drawStage.hidden = true;
  dom.resultStage.hidden = false;
  // eslint-disable-next-line no-unused-expressions
  dom.resultStage.offsetWidth;
  dom.resultStage.classList.add('is-active');
  if (!alreadyDrawn) {
    app.storage.set('lastReading', { date: today, cardIdx: currentIndices.cardIdx }).catch(() => {});
    currentDrawn = true;
  }
  pickInFlight = false;
}

function paintResult(f) {
  dom.btnShare.hidden = false;

  const idx = f.card._index = (CARD_VISUALS.indexOf({ symbol: f.card.symbol, tone: f.card.tone }) + 1) || 0;
  // Use stable index from currentIndices instead â?cleaner.
  const stableIdx = (currentIndices ? currentIndices.cardIdx : 0) + 1;
  dom.cardIndex.textContent = `No. ${String(stableIdx).padStart(2, '0')}`;
  dom.cardTag.textContent = f.card.tag;
  dom.cardArt.textContent = f.card.symbol;
  dom.cardName.textContent = f.card.name;
  dom.cardKeyword.textContent = f.card.keyword;
  dom.cardQuote.textContent = f.quote;
  if (dom.cardInsight) {
    dom.cardInsight.innerHTML = '';
    const label = document.createElement('span');
    label.className = 'card-front__insight-label';
    label.textContent = ui('todayInsightLabel');
    const text = document.createElement('span');
    text.className = 'card-front__insight-text';
    text.textContent = f.insight;
    dom.cardInsight.appendChild(label);
    dom.cardInsight.appendChild(text);
  }
  applySceneTone(f.card.tone);

  dom.fortunes.innerHTML = '';
  for (const item of f.fortunes) {
    const li = document.createElement('li');
    li.className = 'fortune';
    li.innerHTML = `
      <span class="fortune__label">${escapeHtml(item.label)}</span>
      <span class="fortune__bar"><span class="fortune__fill" style="width:0"></span></span>
      <span class="fortune__stars">${'â?.repeat(item.stars)}<span class="ghost">${'â?.repeat(5 - item.stars)}</span></span>
    `;
    dom.fortunes.appendChild(li);
    requestAnimationFrame(() => {
      li.querySelector('.fortune__fill').style.width = `${item.stars * 20}%`;
    });
  }

  dom.suitGood.innerHTML = f.goods.map((s) => `<li>${escapeHtml(s)}</li>`).join('');
  dom.suitBad.innerHTML = f.bads.map((s) => `<li>${escapeHtml(s)}</li>`).join('');

  dom.luckyColorSwatch.style.background = f.color.hex;
  dom.luckyColorName.textContent = f.color.name;
  dom.luckyNumber.textContent = String(f.luckyNumber);
  dom.luckyHour.textContent = f.hour;
  dom.luckyMantra.textContent = f.mantra;
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

dom.btnShare.addEventListener('click', async () => {
  if (!currentIndices) return;
  const f = localizeFortune(currentIndices);
  const lines = [];
  lines.push(ui('shareCardLine')(f.card.name, f.card.keyword));
  lines.push(f.quote);
  if (f.insight) lines.push(ui('shareInsight')(f.insight));
  lines.push('');
  for (const item of f.fortunes) {
    lines.push(`${item.label}: ${'â?.repeat(item.stars)}${'â?.repeat(5 - item.stars)}`);
  }
  lines.push('');
  lines.push(ui('shareGood')(f.goods));
  lines.push(ui('shareBad')(f.bads));
  lines.push('');
  lines.push(ui('shareLucky')(f.color.name, f.luckyNumber, f.hour));
  lines.push(ui('shareMantra')(f.mantra));
  const text = lines.join('\n');
  try {
    await app.clipboard.writeText(text);
    showToast(ui('toastCopied'));
  } catch (_e) {
    showToast(ui('toastCopyFailed'));
  }
});

let toastTimer = null;
function showToast(msg) {
  dom.toast.textContent = msg;
  dom.toast.hidden = false;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { dom.toast.hidden = true; }, 1600);
}

init();
