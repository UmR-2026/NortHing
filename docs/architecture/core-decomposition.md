# northhing Core æè§£æ¶æ

æ¬ææ¦æ¬ northhing core runtime æè§£çä¸¤ä¸ªç¨³å®è®¾è®¡ç»´åº¦ï¼**åå§ç¶æ?*å?*ç®æ ç¶æ?*ã?
åå§ç¶ææè¿°è®¾è®¡å»ºç«æ¶çäºå®æ¶æãè¦åå³ç³»åä¸»è¦é®é¢ï¼ç®æ ç¶ææè¿°ææåå±ãç¨³å®æ¥å£ã?
å®ç°å½å±ãç»è£è¾¹çãä¾èµæ¹ååé£é©çº¦æã?

æ¬æèç¦è®¾è®¡ç»è®ºãè¯¦ç»æ¥å£ãcrate åé¨æ¨¡ååæµè¯è®¾è®¡è§
[`agent-runtime-services-design.md`](agent-runtime-services-design.md)ã?

## 1. èæ¯ä¸ç®æ ?

è®¾è®¡å»ºç«æ¶ï¼northhing å·²ç»ä»?`northhing-core` ä¸­æ½åºäºè¥å¹² owner crateï¼ä½ `northhing-core` ä»æ¿æå¼å®?facadeã?
å®æ´äº§å runtime ç»è£ãagent loopãservice æ¥çº¿ãtool materialization åé¨å?product domain
adapterãè¿ä¸ªå½¢æå¨åè½ä¸å¯è¿è¡ï¼ä½ä¼è®© runtime æè§£æç»­é¢ä¸´ä¸ä¸ªé®é¢ï¼?

- äº§åé»è¾ãå¹³å°æ¥å¥åå·ä½ service å®ç°è¾¹çä¸å¤ç¨³å®ã?
- DesktopãCLIãServerãRemoteãACPãWeb ç­äº§åå½¢æå®¹æè¢«å®æ´ `northhing-core` çµå¼ã?
- ToolãMCPãACPãsubagentãskillsãharness ç­æ©å±ç¹ç¼ºå°ç»ä¸çåå±å½å±ã?

ç®æ å½¢æä¸æ¯å¨ `northhing-core` åç»§ç»­æ©å¼ å®æ?`AgentRuntime`ï¼èæ¯å½¢æå¯ç¬ç«åµå¥ç
Agent Runtime SDKãç¨³å®å¥çº¦å®ä¹ä¸å±å¯ä¾èµçæ¥å£ï¼Product Assembly è´è´£æ³¨åå·ä½å®ç°ï¼?
Runtime ServicesãTool primitives å?Harness Layer åå«éç¦» serviceãtoolãå·¥ä½æµåäº§åå½¢æå·®å¼ã?

Agent Runtime SDK å¨æ¬æä¸­ä¸æ¯æä¸ª crate çç®åéå½åï¼èæ¯ä¸ç»å¯å¯¹å¤ç¨³å®æ¿è¯ºçè¿è¡æ¶è½åè¾¹çã?
ç®æ ç¶æä¸ï¼è°ç¨æ¹åºè½éè¿ç¨³å® API åå»º runtimeãæäº?turnãæ¶è´¹äºä»¶æµãæ³¨å?tool / harness / service
providerãå¤ç?permission / cancellation / persistence / telemetryï¼èä¸éè¦ä¾èµ?`northhing-core`ãapp crateã?
Tauri handle æä»»ä½äº§åå½¢æç concrete managerãå¨è¯¥ç®æ è¾¾æåï¼`execution` å±åªè½ç§°ä¸ºæ§è¡åè¯­éåï¼
ä¸è½å¯¹å¤å®£ç§°ä¸ºå®æ?SDKã?

ç®æ ç¶æå¿é¡»ä¿æäº§åè¡ä¸ºãé»è®¤è½åéåãæéè¯­ä¹ãå·¥å·æåãäºä»¶è¯­ä¹å release æå»ºå½¢æç­ä»·ã?

## 2. æ¶æåå

- ä¾èµåªè½ä»äº§åå¥å?/ äº§åç»è£æµåäº§åè½åãå·ä½ééãæå¡åæ§è¡åè¯­ï¼åæµåç¨³å®å¥çº¦ï¼ä¸å±ä¸å¾æç¥ä¸å±äº§åå½¢æã?
- æ¥å£åå®ç°å¿é¡»åå¼ï¼æ¥å£å±äºç¨³å®å¥çº¦ãRuntime ServicesãTool primitives æ?Harness contractï¼?
  å·ä½å®ç°å±äº Product Assembly çæ³¨åè¾¹çãAdapters æ?Servicesã?
- Product interface å¯ä»¥æå·®å¼ï¼capability contract å¿é¡»æ¶æãä¸åäº§åå¥å£å¯ä»¥éæ©ä¸åè½åéåï¼?
  ä½ä¸è½éè¿ä¸æ² UIãå½ä»¤æåè®®é»è¾æ¥æ¢åå¤ç¨ã?
- `northhing-core` ä¿çå¼å®¹ facade å?`product-full` ç»è£è¾¹çï¼æ° owner crate ä¸å¾ä¾èµå?
  `northhing-core`ã?
- å¯¹å¤ SDK API å¿é¡»æ¯ç¨³å®ãçªå£å¾ãå¯çæ¬åç faÃ§adeï¼ä¸å¾æ `northhing-core`ã`product-full`ãå¨é?
  service bundle æäº§ååé?manager æ´é²ç»è°ç¨æ¹ã?
- Hook æ¯åæ§æ©å±ç¹ï¼Event æ¯äºå®éç¥ãè½æ¹åè¡ä¸ºç?hook å¿é¡»æé¡ºåºãtimeoutãéè¯¯ç­ç¥åç­ä»·ä¿æ¤ã?
- feature group æ¯æå»ºè¾¹çï¼CapabilitySet æ¯äº§åè¿è¡æ¶è½åè¾¹çï¼ä¸¤èå¿é¡»ç± Product Assembly
  æ¾å¼æ å°ã?

## 3. åå§ç¶æé»è¾è§å¾

åå§ç¶æçæ ¸å¿äºå®æ¯ï¼å¤ä¸ª crate å·²ç»æ¿æ¥äºç¨³å®ç±»åãäºä»¶ãstreamãtool contractãé¨å?service
helper å?product domain çº¯é»è¾ï¼ä½å®æ´è¿è¡æ¶ä»ä»?`northhing-core` ä¸ºä¸­å¿ã?

```mermaid
flowchart TB
  Surfaces["äº§åå¥å£<br/>Desktop / CLI / Server / Relay / Remote / Web"]
  Core["northhing-core<br/>å¼å®¹ facade + å®æ´äº§å runtime ç»è£"]
  Acp["northhing-acp<br/>ACP protocol surface / client behavior"]
  Transport["transport / api-layer<br/>API ä¸ä¼ è¾?adapter"]
  CoreTypes["northhing-core-types<br/>ç¨³å® DTO å­é"]
  Events["northhing-events<br/>äºä»¶äºå®ä¸?emitter æ½è±¡"]
  Ports["northhing-runtime-ports<br/>trait-only runtime è¾¹ç"]
  Stream["northhing-agent-stream<br/>stream èå"]
  AgentTools["northhing-agent-tools<br/>tool contract ä¸çº¯ç­ç¥"]
  ToolRuntime["tool-execution<br/>tool-runtime package / ä½å± helper"]
  ToolPacks["tool-provider-groups<br/>northhing-tool-packs package / provider plan"]
  ServicesCore["northhing-services-core<br/>åºç¡ service helper / filesystem facade"]
  ServicesIntegrations["northhing-services-integrations<br/>MCP / Git / Remote helper owner"]
  ProductDomains["northhing-product-domains<br/>MiniApp / function-agent çº?domain"]
  Terminal["terminal-core<br/>terminal domain"]
  Ai["northhing-ai-adapters<br/>æ¨¡å provider adapter"]
  External["å¤é¨ç³»ç»<br/>OS / Git / MCP / ACP / AI provider / remote host"]

  Surfaces --> Core
  Surfaces --> Transport
  Surfaces --> Acp
  Acp --> Core
  Core --> CoreTypes
  Core --> Events
  Core --> Ports
  Core --> Stream
  Core --> AgentTools
  Core --> ToolRuntime
  Core --> ToolPacks
  Core --> ServicesCore
  Core --> ServicesIntegrations
  Core --> ProductDomains
  Core --> Terminal
  Core --> Ai
  Core --> Transport
  ServicesCore --> External
  ServicesIntegrations --> External
  Terminal --> External
  Ai --> External
```

åå§ç¶æä¸»è¦æ¨¡åèå´ï¼

| æ¨¡å | åå§å®ä½ | æ¶æå½±å |
|---|---|---|
| `northhing-core` | å¼å®¹ facadeãagent runtimeãtool runtime ç»è£ãservice æ¥çº¿åå®æ´äº§åè½åéå?| ä»æ¯äºå®ä¸ç runtime ownerï¼æè§£å¿é¡»åä¿æ¤è¡ä¸ºç­ä»· |
| `northhing-runtime-ports` | é¢å runtime/service è¾¹çç?DTO å?trait | åªå®ä¹?contractï¼ä¸æ¥æ runtime å®ç° |
| `tool-contracts` / `northhing-agent-tools` | provider-neutral tool DTOãmanifestãpath/result policyãcatalog contract å?deterministic execution admission gate | éåæ¿æ¥çº?tool contract ç­ç¥ï¼ä½ä¸åºæ¥æå·ä½ IO tool |
| `tool-execution` / `tool-runtime` | æ¢æä½å±å·¥å·æ§è¡ helper crate | ç®æ æ¯åªæ¿æ¥ä½å± file/search/tool execution helperï¼ä¸æ¥æäº§å registry æ?permission policy |
| `northhing-services-core` | åºç¡ service helperãæ¬å?filesystem facadeãé¨åéç¨ service é»è¾ | éåä½ä¸ºæ¬å°åºç¡ service ownerï¼ä½ä¸è½å¸æ¶äº§å runtime è¯­ä¹ |
| `northhing-services-integrations` | MCPãGitãremote-connectãremote-SSH ç­?integration helper | éåæ¥æå¤é¨åè®®åéä¾èµ service implementationï¼ä¸åºååæç¥äº§å?interface |
| `northhing-product-domains` | MiniAppãfunction-agent ç­çº¯ç¶æãç­ç¥ãport åé¨åå³ç­é»è¾ | éåæ¿æ¥ pure domainï¼ä¸åºç´æ¥æ§è¡?filesystem/Git/AI concrete call |
| `northhing-acp` | ACP protocol interface å?client behavior | åºä¿æäº§ååè®®å¥å£ï¼ä¸ä¸æ²å° Agent Runtime |
| `transport` / `api-layer` | surface å?runtime ç?API/transport adapter | åºä¿æä¼ è¾å±ï¼ä¸æ¥æ runtime owner |

## 4. åå§ç¶æä¸»è¦é®é¢?

### 4.1 åå±ä¸æ¸æ?

åä¸è½åç»å¸¸åæ¶åå« UI/commandãruntime orchestrationãtool executionãservice IO å?domain
decisionãåå§ç¶æä»£ç ä¸­è¿äºé¨åä»å¤§ééè¿ `northhing-core` ä¸²èï¼å¯¼è´æè§£æ¶é¾ä»¥å¤æ­âç§»å¨çæ¯æ¥å£ã?
å®ç°ãç»è£é»è¾è¿æ¯äº§åè¡ä¸ºâã?

### 4.2 æ¥å£ä¸å®ç°è¾¹çä¸ç¨³å®

å·²æ `runtime-ports` åè¥å¹?contract crateï¼ä½è®¸å¤ call site ä»ä¾èµ?concrete managerã?
core-owned context æå®æ?product runtime snapshotãæ¥å£æ²¡æç¨³å®å°è¶³ä»¥è®?runtime ä¸å·ä½?service
å®ç°ç¬ç«æ¼è¿ã?

### 4.3 äº§åå½¢æè¢«å®æ´ core çµå¼

DesktopãCLIãServerãRemoteãACP å?Web çå¥å£å·®å¼è¾å¤§ï¼ä½åå§ç¶æä¸å¤§å¤ä»éè¿å®æ´ `northhing-core`
è·å¾è½åãè¿ä¼è®©è½»éäº¤ä»å½¢æç»§æ¿ä¸å¿è¦ç?toolãserviceãUI æå¹³å°ä¾èµã?

### 4.4 Tool contract ä¸?tool execution æ··å

provider-neutral manifestãpath policyãresult policyã`ToolUseContext` runtime handleãcollapsed unlock
lifecycleãruntime artifact persistence å?product registry materialization å¨åå§ç¶æä¸ä¸?concrete tool
execution äº¤ç»å?core åå¶å¼å®¹è·¯å¾ä¸­ãç®æ ç¶æä¸ï¼tool contracts åºæ¥æ?provider-neutral manifest /
catalog / permission / result / artifact contractï¼coreãservices æ?adapter åªä¿çå®é?IO tool adapterã?
state updateãæ§è·¯å¾ facade åæç­ä»·ä¿æ¤çæè§£è¾¹çãå·¥å?owner æè§£å¦ææ²¡æå¿«ç§ä¿æ¤ï¼å®¹ææ¹å?
prompt-visible manifestã`GetToolSpec`ãMCP/ACP catalog æ?oversized result è¡ä¸ºã?

### 4.5 ServiceãMCPãACP ä¸?runtime kernel å®¹æäº¤å

MCP å?ACP æ¯å¤é¨åè®?è½åæ¥å¥ï¼ä¸åºåæ?Agent Runtime SDK çåé¨åè®®ä¾èµãRuntime kernel åªåºçè§
external capabilityãtool provider æ?service portï¼è¿æ¥çå½å¨æãé´æãtransport å?timeout ç­ç¥åºç±
AdaptersãServices æ?Product Assembly ç®¡çã?

### 4.6 æ©å±ç¹ç¼ºå°ç»ä¸è¯­ä¹

agent definitionsãsubagentsãskillsãprompt modulesãtool providersãMCP providersãhooks å?
product commands é½æ¯æ©å±ç¹ï¼ä½ç®åæ²¡æç»ä¸è¡¨è¾¾å®ä»¬åå«å±äºåªä¸å±ãå¦ä½æ³¨åãæ¯å¦åè®¸æ¹åè¡ä¸ºã?
ä»¥åå¦ä½åæéåæµè¯ä¿æ¤ã?

### 4.7 feature graph è¿ä¸æ¯äº§åè½åç©é?

åå§ç¶æä¸ï¼`product-full` æ¯å®æ´äº§åè½åçå®å¨ç½ï¼ä¸æ¯æç»æäº§åæåç?feature matrixãç´æ¥åè½»é»è®?feature
ææ feature group å½æäº§åè½åè¾¹çï¼é½ä¼å¼å¥æå»ºå½¢æååå¸è½åæ¼ç§»ã?

### 4.8 æå»ºä¸æµè¯çµå¼è¿å¤?

éä¾èµåå®æ´ runtime èåå?`northhing-core` å¨å´ï¼å¯¼è´å±é¨æµè¯ãowner crate æµè¯åè½»éäº§åå¥å£å®¹æè¢«
ä¸ç¸å³ä¾èµæå¥ç¼è¯åé¾æ¥è·¯å¾ãç®æ ç¶æå¿é¡»è®©ä¾èµæ¶çå¯åº¦éï¼åæ¶ä¸è½ä»¥çºç²åè½ç­ä»·æ¢åæå»ºæ¶çã?

### 4.9 SDK åå¸è¾¹çä¸è¶³

å·²æ `northhing-agent-runtime`ã`northhing-runtime-services`ã`tool-contracts`ã`tool-execution`ã`northhing-harness`
å?`runtime-ports` ç­?SDK åéåè¯­ï¼ä½ç¼ºå°å¯å¯¹å¤æ¿è¯ºçç»ä¸ runtime faÃ§adeãç¨³å®éè¯¯æ¨¡åãäºä»¶æµåè®®ã?
provider æ³¨åè¾¹çãæä¹å/æ¢å¤å¥çº¦åæå°ä¾èµæå»ºå½¢æãå¦æå¤é¨è°ç¨æ¹ä»éè¦ç´æ¥çè§?`northhing-core`ã?
`product-full`ãconcrete service manager æäº§åå½ä»¤è·¯å¾ï¼è¯´æ SDK è¾¹çå°æªå®æã?

## 5. å¯¹ç§åæ

æ¬èåªæç¼å¯¹ northhing åå±æç¨çæ¶æä¿¡å·ï¼ä¸æå¶ä»é¡¹ç®çå®ç°å½¢æç´æ¥å¤å¶å° northhingã?

### 5.1 Claude Code ç¸å³å®ç°åè?

Claude Code ç¸å³ Rust å®ç°åèä¸­ï¼workspace å°?CLI binaryãprovider APIãruntimeãtoolsã?
commandsãpluginsãtelemetry å?mock harness ææä¸å crateãå¶ `runtime` è´è´£ sessionãconfigã?
permissionãMCPãprompt å?runtime loopï¼`tools` è´è´£ tool specs ä¸æ§è¡ï¼`commands` è´è´£ slash command
registryï¼`plugins` è´è´£ plugin metadataãhook å?install/enable/disable surfacesãè¯¥ç»æè¯´æï¼?

- å·¥å·è§æ ¼ãå½ä»?surfaceãplugin/hook å?runtime loop å¯ä»¥åå¼æ¼è¿ã?
- permissionãMCP lifecycleãtask registryãLSP registry ç­å¯ä½ä¸º runtime/service owner ç®¡çï¼èä¸æ¯æ£è½å¨ UIã?
- å¦æ runtime crate åæ¶å¸æ¶ sessionãMCPãpermissionãprompt å?tool bridgeï¼ä¹ä¼åææ°çéèåç¹ã?

æ»ç»ï¼æå?crate ä¸æ¯ç®æ æ¬èº«ï¼å³é®æ¯è®?CLI/TUIãcommandsãtoolsãpluginsãruntime å?
service integrations éè¿ç¨³å® contract ç»åï¼é¿åæ `northhing-core` çèåé®é¢æ¬å°æ°ç?runtime crateã?

### 5.2 Opencode

Opencode å®æ¹ææ¡£å±ç¤ºäºæ´åäº§ååçæ©å±æ¨¡åï¼åä¸ä¸?agent å¯ä»¥è¿è¡å?terminalãdesktop æ?IDEï¼?
agents åä¸º primary agents å?subagentsï¼å¯éç½® promptãmodel ä¸?tool accessï¼tools éè¿ permission æ§å¶ï¼?
å¹¶å¯éè¿ custom tools æ?MCP servers æ©å±ï¼plugins è®¢é commandãfileãpermissionãsessionãtoolãTUI
ç­äºä»¶ï¼skills éè¿ç¬ç«ç®å½æéåç°åå è½½ã?

æ»ç»ï¼?

- AgentãToolãMCPãPlugin/HookãSkill å?Product Surface åºè¯¥æ¯äºç¸è¿æ¥çæ©å±é¢ï¼èä¸æ¯åä¸ä¸ªæ¨¡ååé¨çåæ¯ã?
- æéåå·¥å·å¯è§æ§å¿é¡»æ¯ runtime å¯è§æµç contractï¼ä¸è½åªå­å¨äº?UI æ?prompt æ¼æ¥ä¸­ã?
- å¤äº§åå½¢æéè¦?Product Assembly å?capability/provider éæ©ï¼èä¸æ¯è®© Agent Runtime SDK å¤æ­è°ç¨æ¥èª
  DesktopãCLIãRemote è¿æ¯ ACPã?

## 6. ç®æ é»è¾è§å¾

ç®æ æ¶æä»¥å­ä¸ªç©ç?owner ååºè¡¨è¾¾ä¾èµæ¹åã`interfaces` åªæ¿è½½åè®®åå®¿ä¸»å¥å£ï¼`assembly` è´è´£äº§åè½åéæ©ä¸æ³¨åï¼`adapters` è´è´£åè®®ãtransport åå¤é?provider è½¬æ¢ï¼`services` è´è´£æ¬å°ç³»ç»ä¸?runtime infrastructure çå¯å¤ç¨å·ä½å®ç°ï¼`execution` åªæ¾å¯ç§»æ¤æ§è¡åè¯­ï¼`contracts` æä¾ç¨³å®äºå®ãport åäº§åé¢åè§åãè¿æ ·å¯ä»¥åæ¶åºåâåè®®ééâåâæå¡å®ç°âï¼ä¹é¿åæ execution è¯¯è§£ä¸ºå®æ´è¿è¡æ¶å®ç°å±ã?

```mermaid
flowchart TB
  Interfaces["æ¥å£ä¸å¥å£å±ï¼Interfaces and Entrypointsï¼?br/>UI / command / protocol interface / delivery profile"]
  Assembly["äº§åç»è£å±ï¼Product Assemblyï¼?br/>compatibility facade / capability selection / adapter and service registration"]
  Adapters["ééå±ï¼Adaptersï¼?br/>AI / API / transport / WebDriver / external provider translation"]
  Services["æå¡å®ç°å±ï¼Servicesï¼?br/>filesystem / git / terminal / MCP / remote / process / OS integration"]
  Execution["æ§è¡åè¯­å±ï¼Execution Primitivesï¼?br/>agent / harness / stream / typed-service / tool primitives"]
  Contracts["ç¨³å®å¥çº¦ä¸äº§åé¢åå±ï¼Stable Contracts and Product Domainsï¼?br/>DTO / event / runtime port / product domain policy"]
  External["å¤é¨ç³»ç»ï¼External Systemsï¼?br/>OS / Git / MCP server / ACP client / AI provider / remote host"]

  Interfaces --> Assembly
  Interfaces --> Adapters
  Assembly --> Adapters
  Assembly --> Services
  Assembly --> Execution
  Assembly --> Contracts
  Adapters --> Services
  Adapters --> Execution
  Adapters --> Contracts
  Services --> Execution
  Services --> Contracts
  Execution --> Contracts
  Adapters --> External
  Services --> External
```

ä¾èµæ¹ååªåè®¸ä»ä¸å°ä¸ãæ¥å£ä¸å¥å£å±æ´é²äº§åå½¢æï¼ç»è£å±éæ©è½åéåå¹¶æ³¨å?adapter/serviceï¼ééå±ç¿»è¯åè®®åå¤é¨ providerï¼æå¡å®ç°å±æ¥è§¦ OSãprocessãfilesystemãgitãterminalãMCP å?remoteï¼æ§è¡åè¯­å±æä¾å¯å¤ç?runtime building blocksï¼å¥çº¦å±æä¾ç¨³å®äºå®ãport åäº§åé¢åè§åãä»»ä½ä¸å±?crate ååè¯»åäº§åå¥å£ãç»è£éç½®æ host state é½è§ä¸ºè¾¹çè¿è§ã?

## 7. ç®æ å±çº§

ç®æ å±çº§ä»¥ç©ç?owner ååºä¸ºå¥å£ãæ¯ä¸ªååºå¯ä»¥åå«å¤ä¸?crateï¼ä½ crate åé¨èè´£å¿é¡»è½å¤éè¿ä¾èµãæµè¯åè¾¹çèæ¬ç¬ç«éªè¯ã?

### 7.1 æ¥å£ä¸å¥å£å±ï¼Interfaces and Entrypointsï¼?

æ¥å£ä¸å¥å£å±æ¯ç¨æ·ãåè®®æå¤é¨ç³»ç»è¿å¥ northhing çå¥å£ï¼è´è´£ UIãå½ä»¤ãè·¯ç±ãåè®®æ¥å£ãäº¤ä»å½¢æéæ©å?host integrationãå¯¹åºèå´åæ?`src/apps/*`ã`src/web-ui`ã`src/mobile-web`ã`northhing-Installer`ã`tests/e2e` å?`src/crates/interfaces`ãå¥å£å±å¯ä»¥éæ© `DeliveryProfile` å¹¶è°ç?assembly æ?adapter APIï¼ä½ä¸æ¥æå±äº?runtime è¡ä¸ºã?

### 7.2 äº§åç»è£å±ï¼Product Assemblyï¼?

äº§åç»è£å±è´è´£å¼å®¹å¯¼åºãå®æ´äº§åè½åéæ©ãfeature group å?capability set çæ å°ãadapter/service æ³¨åå?product-full æ¥çº¿ãç©çä½ç½®æ¯ `src/crates/assembly`ï¼å½ååå?`northhing-core` å¼å®¹é¨é¢å?`northhing-product-capabilities` è½åæ¨¡åã`product-capabilities` åªæè¿?capability idãtool groupãservice requirement å?harness selectionï¼ä¸æ§è¡ IOï¼ä¹ä¸æ¿è½½äº§åé¢åç¶ææºã?

### 7.3 ééå±ï¼Adaptersï¼?

ééå±è´è´£åè®®ãtransportãå¤é?provider åå®¿ä¸»éä¿¡è½¬æ¢ï¼ç©çä½ç½®æ¯ `src/crates/adapters`ãå¶ä¸?`ai-adapters` è´è´£ AI provider è¯·æ±/ååºæ å°å?provider stream åè®®è§£æï¼è§£æç»æåºè½¬æ¢ä¸?execution å±æ¥æçç»ä¸ stream å¥çº¦ï¼`api-layer` è´è´£äº§åå®¿ä¸»å±ç¨çåç«?API adapterï¼`transport` è´è´£äºä»¶æéå host transport adapterï¼`webdriver` è´è´£ WebDriver åè®®åæµè§å¨èªå¨å?adapterãééå±ä¸æ¥æäº§åè½åéæ©ï¼ä¹ä¸æ¿è½½å¯å¤ç¨ OS service å®ç°ã?

### 7.4 æå¡å®ç°å±ï¼Servicesï¼?

æå¡å®ç°å±è´è´£æ¥è§¦æ¬å°ç³»ç»å runtime infrastructure çå¯å¤ç¨å·ä½å®ç°ï¼ç©çä½ç½®æ¯ `src/crates/services`ãå¶ä¸?`services-core` æ¿è½½è½»é service primitiveï¼`services-integrations` æ¿è½½ MCPãGitãremoteãfile watch åäº§åé¢å?port çå·ä½å®ç°ï¼`terminal` æ¿è½½ PTYãshell integration å?terminal session infrastructureãæå¡å®ç°å±å¯ä»¥å®ç° `contracts`ã`execution` æ?`product-domains` å®ä¹ç?portï¼ä½ä¸éæ©äº§å profileï¼ä¹ä¸ç´æ¥æ´é?UI/åè®®å¥å£ã?

### 7.5 æ§è¡åè¯­å±ï¼Execution Primitivesï¼?

æ§è¡åè¯­å±æä¾?provider-neutral ç?runtime building blocksï¼ç©çä½ç½®æ¯ `src/crates/execution`ã`agent-runtime`ã`agent-stream`ã`harness`ã`runtime-services`ã`tool-contracts`ã`tool-provider-groups` å?`tool-execution` åå«å®ä¹ agent loop factsãç»ä¸ stream DTO / tool-call ç´¯ç§¯ / replay å¥çº¦ãworkflow descriptorãtyped service bundleãtool manifest / permission / result policyãtool group facts åä½å±?tool execution helperãå½å?Cargo package / lib åä¿æå¼å®¹ï¼ä½ç©çç®å½æèè´£å½åãå®ä»¬åªè½ä¾èµç¨³å®å¥çº¦ææç¡®ç?provider-neutral DTOï¼ä¸ç´æ¥åå»º Tauri handleãfilesystem managerãGit providerãMCP clientãAI client æ?host processã?

### 7.6 ç¨³å®å¥çº¦ä¸äº§åé¢åå±ï¼Stable Contracts and Product Domainsï¼?

ç¨³å®å¥çº¦ä¸äº§åé¢åå±æ¯æä½å±ï¼ç©çä½ç½®æ¯ `src/crates/contracts`ãå®åå« `core-types`ã`events`ã`runtime-ports` å?`product-domains`ã`product-domains` æ?Product Domain Modelï¼è´è´?MiniAppãfunction-agent ç­é¢å?DTOãçº¯ç­ç¥ãç¶æè§ååçª?portï¼å·ä½?GitãfilesystemãAI æ?worker execution å®ç°å?servicesãadapters æ?assembly/core çå¼å®¹è·¯å¾ä¸­ï¼ä¸å¾åæµå° contractsã?

### 7.7 æ©å±ç¹å½å±?

- AIãAPIãtransport å?WebDriver çåè®®è½¬æ¢å±äº?Adaptersã?
- MCPãterminalãfilesystemãgitãremote å?file watch çå¯å¤ç¨å·ä½å®ç°å±äº Servicesã?
- Tool manifestãpermissionãexecution admissionãresult / artifact policy å±äº Execution Primitives ç?`tool-contracts`ã?
- Tool provider group facts å±äº Execution Primitives ç?`tool-provider-groups`ï¼ä½å±?filesystem/search helper å±äº `tool-execution`ã?
- Agentãsubagentãprompt moduleãschedulerãsession / turn facts å?hook routing å±äº Execution Primitivesã?
- Harness workflow descriptor å?route plan å±äº Execution Primitivesï¼å·ä½å·¥ä½æµ IO çå¨ ServicesãAdapters æå¼å®¹è·¯å¾ï¼ç´å°æç­ä»·ä¿æ¤ååè¿ç§»ã?
- Capability packãdelivery profileãadapter/service selection å?product-full assembly å±äº Product Assemblyã?
- äº§åé¢åç¶æãè§åãport å?domain policy å±äº Stable Contracts and Product Domainsã?

## 8. æ¥å£ä¸å®ç°å³ç³?

æ¥å£ç±ç¨³å®å¥çº¦ãRuntime ServicesãTool Contracts æ?Harness contract å®ä¹ï¼å·ä½å®ç°ç± adapterãservice æäº§åå¥å£åå»ºï¼æ³¨åå¨ä½åªè½åçå?Product AssemblyãAgent Runtimeãtool contractsãtool execution å?Harness åªæ¥æ¶å·²ç»ç»è£å¥½çæ¥å£æ provider registryï¼ä¸ç´æ¥åå»ºå¹³å°å®ç°ã?

```mermaid
flowchart TB
  Interface["æ¥å£ä¸å¥å£å±ï¼Interfaces and Entrypointsï¼?br/>éæ©å¥å£å?DeliveryProfile"]
  Assembly["äº§åç»è£å±ï¼Product Assemblyï¼?br/>å¯ä¸æ³¨åç?]
  ServiceBuilder["è¿è¡æ¶æå¡å±ï¼Runtime Servicesï¼?br/>RuntimeServicesBuilder"]
  ToolBuilder["å·¥å·æ§è¡åè¯­ï¼Tool Primitivesï¼?br/>tool contracts / groups / execution"]
  HarnessBuilder["å·¥ä½æµç¼æå±ï¼Harness Layerï¼?br/>HarnessRegistryBuilder"]
  AgentRegistry["Agent æ§è¡åè¯­ï¼Agent Runtimeï¼?br/>AgentDefinitionRegistry"]
  CommandRegistry["æ¥å£ / äº§åç»è£å±?br/>ProductCommandRegistry"]
  Runtime["Agent / Tool / Harness primitives<br/>åªæ¶è´¹æ¥å?]
  Adapters["ééå±ï¼Adaptersï¼?br/>AI / API / transport / WebDriver adapters"]
  Services["æå¡å®ç°å±ï¼Servicesï¼?br/>OS / filesystem / Git / terminal / MCP / remote services"]
  Contracts["ç¨³å®å¥çº¦ä¸äº§åé¢åå±ï¼Stable Contracts and Product Domainsï¼?br/>DTO / event / port trait"]

  Interface --> Assembly
  Assembly --> ServiceBuilder
  Assembly --> ToolBuilder
  Assembly --> HarnessBuilder
  Assembly --> AgentRegistry
  Assembly --> CommandRegistry
  Assembly --> Adapters
  Assembly --> Services
  ServiceBuilder --> Runtime
  ToolBuilder --> Runtime
  HarnessBuilder --> Runtime
  AgentRegistry --> Runtime
  CommandRegistry --> Interface
  Runtime --> Contracts
  Adapters --> Contracts
  Services --> Contracts
  Adapters --> Services
```

æ³¨åå¨ä¸åæç®æ å±çº§çå¯¹åºå³ç³»å¦ä¸ï¼

| æ³¨åå?/ ç»è£ç?| æå±ç®æ å±çº?| åå§æ¿è½½ä¸ç®æ æ¿è½?| æ³¨ååå®¹ |
|---|---|---|---|
| `ProductAssembler` / `ProductAssemblyPlan` | äº§åç»è£å±ï¼Product Assemblyï¼?| åå§å¯å¨ `northhing-core` facade æäº§åå¥å£ï¼ç®æ å¯æ¶æä¸º assembly owner | `DeliveryProfile`ã`CapabilitySet`ãfeature groupãadapter/service éæ© |
| `RuntimeServicesBuilder` | æ§è¡åè¯­å±ï¼Execution Primitivesï¼ä¸æå¡å®ç°å±ï¼Servicesï¼çè¾¹ç | ç®æ å?`northhing-runtime-services`ï¼è¿æ?`northhing-runtime-ports`ã`northhing-services-*` ååå§?service wiring | filesystemãworkspaceãsession storeãGitãterminalãnetworkãMCP catalogãremote connection / workspace / projection port |
| `ToolRuntimeBuilder` | æ§è¡åè¯­å±ï¼Execution Primitivesï¼?| `tool-execution`ã`tool-contracts`ã`tool-provider-groups`ï¼Cargo package åä¿æå¼å®?| tool providerãtool groupãmanifestãpermission gateãtool hook |
| `HarnessRegistryBuilder` | å·¥ä½æµç¼æå±ï¼Harness Layerï¼?| ç®æ å?`northhing-harness`ï¼åå§å¯ç?`northhing-core::agentic::harness` æ³¨å legacy-facade provider | SDDãDeep ReviewãDeepResearchãMiniApp ç­?harness provider |
| `AgentDefinitionRegistry` | æ§è¡åè¯­å±ï¼Execution Primitivesï¼?| ç®æ å?`northhing-agent-runtime`ï¼åå§å¯ç?`northhing-core` agent definition ä»£ç æ¿è½½ | agentãsubagentãprompt moduleãskill definition |
| `ProductCommandRegistry` | æ¥å£ä¸å¥å£å±ï¼Interfaces and Entrypointsï¼ä¸äº§åç»è£å±ï¼Product Assemblyï¼çè¾¹ç | äº§åå¥å£æ?assembly æ¨¡å | è¾å¥æ¡å½ä»¤ãå®¡æ ¸å¥å£ãMiniApp å¥å£å?capability / harness / runtime request çæ å°?|
| adapter set | ééå±ï¼Adaptersï¼?| `northhing-ai-adapters`ã`northhing-api-layer`ã`northhing-transport`ã`northhing-webdriver`ãapp adapters | AIãAPIãtransportãWebDriver ç­åè®®æå¤é¨ provider adapter |
| service set | æå¡å®ç°å±ï¼Servicesï¼?| `northhing-services-*`ã`terminal-core` åå·ä½?app service implementations | OSãfilesystemãGitãterminalãMCPãremote çå·ä½?serviceï¼Remote service åé¨ç»§ç»­åºå SSHãrelayãæ¬å°é§éãè¿ç«?OS æ¯æ |

æ³¨åè·¯å¾å¿é¡»æ¯æ¾å¼ãtypedãå¯æµè¯çï¼

- æ¥å£ä¸å¥å£å±ï¼Interfaces and Entrypointsï¼åªéæ© `DeliveryProfile` åäº§åéç½®ï¼ä¸ç´æ¥æ concrete manager ä¼ å¥ runtimeã?
- äº§åç»è£å±ï¼Product Assemblyï¼æ ¹æ®äº§åå½¢æåå»ºææ¥æ¶ adapter/serviceï¼å¹¶è°ç¨ typed builder å®ææ³¨åã?
- ToolãOSãRemoteãProtocol provider åå«çå¨å¯¹åº appãAdapters æ?Services ä¸­ï¼éè¿åä¸ç»?port æ´é²ã?
- Tauri åªè½åºç°å?Desktop appãtransport/API adapter æäº§åå¥å£å½ä»¤å¤è§ä¸­ï¼Agent Runtimeã?
  Tool primitivesãHarnessãRuntime Services contract å?Product Capabilities ä¸å¾ä¾èµ Tauri handleã?
  windowãcommand macro æ?desktop app stateã?
- Remote provider å¿é¡»æåç¨³å®è¿æ¥æ¥å£åå·ä½è¿ç«?OS / transport å®ç°ï¼é¿åæ SSHãrelay æè¿ç«¯å¹³å°å·®å¼æ³æ¼å° runtimeã?
- ä¸æ¯æçè½åå?assembly ç?capability availability ä¸­æ¾å¼è¿å?unsupported / unavailableï¼ä¸å?execution primitive ååäº§ååæ¯ã?
- ç¦æ­¢ä½¿ç¨æ ç±»å?`Any` service locatorãå¨å± mutable registry æä¸å±?crate ååè¯»åäº§åéç½®ã?

## 9. é£é©

| é£é© | ä¿æ¤æ¹å¼ |
|---|---|
| äº§åç»è£å±ï¼Product Assemblyï¼è¨èä¸ºæ°çå¨å±ç¶æä¸­å¿?| assembly åªåæå»ºææ³¨åï¼è¾åºä¸å¯å?runtime partsï¼äº§åç¶æä»å½?surface æ?runtime owner |
| æ¥å£æå¾è¿ç»ï¼å¯¼è´å¤æåº¦åå¨æååææ¬ä¸å?| ä»?capability åç¨³å®ç¨ä¾å®ä¹?port ç²åº¦ï¼ç­è·¯å¾é¿åè¿è¡æ?map lookupï¼ä¼å?builder-time æ³¨å¥ |
| å¹³å°å®ç°æ³æ¼å?AgentãTool æ?Harness execution primitives | ä¾èµæ£æ¥ç¦æ­?execution owner ä¾èµ app crateãTauriãCLI TUIãACP protocol å?concrete service crate |
| core æååä»éå¼ç»å® Tauri | Tauri åªåè®¸å¨ Desktop app ææç¡?feature-gated adapterï¼åä¸å±ä¼ é?typed portãDTOãevent fact å?capability availability |
| ä¸åäº§åå½¢æè½åç©éµæ¼ç§?| Product Assembly ç»´æ¤ capability matrixï¼åå°ææ¿æ¢è½åæ¶è¡¥äº§åå¥å£éªè¯å?unsupported è¡ä¸ºæµè¯ |
| ToolãMCPãACP ç?manifestãpermission æäºä»¶è¯­ä¹æè§£åä¸ç­ä»?| ä¿çæ§è·¯å¾å¼å®?facadeï¼å¢å?manifest snapshotãpermission å³ç­åäºä»¶æ å°ç­ä»·æµè¯?|
| Harness provider åªåæ³¨åä½è¢«è¯¯è®¤ä¸ºå·²ç»æ¥ææ§è¡è¯­ä¹?| descriptor-only / legacy-facade provider åªè½çæ route planï¼æ§è¡è¯­ä¹ç§»å¨å¿é¡»åç¬è¯æè¡ä¸ºç­ä»?|
| `northhing-core` åªæ¯æ¹åä¸ºæ°çå·¨å?runtime crate | æ?owner crate å¿é¡»æåä¸èè´£åæå°ä¾èµï¼äº§åè½åãharnessãservice å®ç°ä¸å¾ç»§ç»­å å¥ agent kernel |
| ç®æ  crate åè¡åå»ºä½æ²¡æçå®?owner | åªæ owner è¾¹çãæ§è·¯å¾å¼å®¹ãfocused testsãä¾èµæ¶çå boundary check åæ¶æç«æ¶æåå»º crateï¼å¦åç»§ç»­çå?facade |

## 10. ç®æ ç¶æå¤å®?

- `northhing-core` ä¸åæ¯äºå®ä¸çå®æ?runtime ownerï¼èæ¯å¼å®¹ facade å?`product-full` ç»è£è¾¹çã?
- Agent Runtime SDK å¯å¨ä¸ä¾èµ?`northhing-core`ãapp crate æ?Tauri çæåµä¸è¢«åµå¥ï¼å¹¶éè¿ç¨³å® builder /
  runner / event stream / registry API æä¾ agent è½åã?
- Agent RuntimeãTool Contracts / Tool Provider Groups / Tool ExecutionãRuntime ServicesãHarness å?Product Capabilities åå«æ¥æå¯å®¡æ¥çèè´£è¾¹çã?
- ç¨³å®å¥çº¦åå execution owner å®ä¹æ¥å£ï¼å·ä½?ToolãOSãRemote service çå¨ Servicesï¼åè®®åå¤é¨ provider è½¬æ¢çå¨ Adaptersã?
- äº§åç»è£å±ï¼Product Assemblyï¼æ¯å¯ä¸æ³¨åç¹ï¼éè¿ typed builder / registry è¿æ¥æ¥å£åå·ä½å®ç°ã?
- Tauri åªå±äº?Desktop app ææç¡?feature-gated adapterï¼ä¸è¿å¥ coreãexecution owner æ?contract crateã?
- runtime åªä¾èµ?remote connectionãremote workspaceãremote projection å?capability facts ç­?portï¼SSHãrelayã?
  æ¬å°é§éãè¿ç«?OS å·®å¼åè®¤è¯æ¹å¼å±äºå·ä½?Remote providerã?
- äº§åå½¢æå·®å¼éè¿ capability matrix å?Product Assembly è¡¨è¾¾ï¼ä¸éè¿ä¸æ² UIãå½ä»¤ãåè®®æå¹³å°å®ç°è¡¨è¾¾ã?
- æéãå·¥å·æåãäºä»¶ãsessionãremote workspace å?release æå»ºå½¢æå¿é¡»ä¿æåè½ç­ä»·ã?
