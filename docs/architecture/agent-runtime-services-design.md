# Agent Runtime SDK ä¸?Runtime Services è®¾è®¡

æ¬ææ?[`core-decomposition.md`](core-decomposition.md) çå¼åè®¾è®¡ææ¡£ï¼æè¿°ç®æ æ¨¡åã?
æ¥å£ãcrate åé¨ç»æåè¡ä¸ºä¿æ¤ãæ¬æåªè®°å½è®¾è®¡çº¦æï¼ä¸è®°å½å®ç°è¿ç¨æéªè¯è®°å½ã?

## 1. è®¾è®¡ç®æ ä¸è¾¹ç?

- Agent Runtime SDK å¯è¢« DesktopãCLIãServerãRemoteãACP ç­äº§åå½¢æåµå¥ã?
- Agent Runtime SDK å¯¹å¤æä¾ç¨³å®ãçªå£å¾ç?runtime APIï¼èä¸æ¯æ´é?`northhing-core`ãäº§åå½ä»¤è·¯å¾æ concrete managerã?
- Runtime ä¸æç¥å¹³å°å·®å¼ãå·¥å·å®ç°å·®å¼åæå»ºå½¢æå·®å¼ã?
- Tool ä½¿ç¨éç¨æ¥å£å?provider group æ³¨åï¼ä¸ç»å®åºå±å®ç°ã?
- å·ä½ adapter ä¸?service å®ç°ç±ä¸å±?Product Assembly æ³¨å¥ã?
- Harness å¯æ©å±ï¼æ°å¢ SDD ç­å·¥ä½æµä¸ä¾µå?runtime kernelã?
- æ¯ä¸ª crate åªä¾èµæå°ç¨³å®éåï¼ä¾èµæ¹åå¯æ£æ¥ã?

### 1.1 SDK åå¸è¾¹ç

Agent Runtime SDK çåå¸è¾¹çä»¥è°ç¨æ¹è½åä¸ºåï¼èä¸æ¯ä»¥ç©ç crate å½åä¸ºåãè¾¾å°ç®æ ç¶ææ¶ï¼å¤é¨è°ç¨æ¹
åºè½å¨ä¸ä¾èµ `northhing-core`ãapp crateãTauri æäº§ååé?manager çæåµä¸å®æä»¥ä¸å¨ä½ï¼?

- æå»º runtimeï¼æ³¨å?model providerã`RuntimeServices`ãtool providerãharness providerãagent definitionsã?
  hooks å?runtime configã?
- åèµ·æ§è¡ï¼åå»ºææ¢å¤ sessionï¼æäº?turnï¼åæ¶?turnï¼æ¶è´?provider-neutral event streamã?
- æ§è¡å·¥å·ï¼éè¿ç¨³å® tool manifestãpermission requestãtool resultãartifact ref å?cancellation contract
  ç®¡çå·¥å·è°ç¨ã?
- æ©å±è½åï¼éè¿ registry æ³¨å subagentãprompt moduleãskillãMCP/API toolãharness workflow å?post-turn
  processorã?
- å¤çè¿ç»´è¯­ä¹ï¼æ¥æ?typed errorãusage/cost/cache factsãtelemetry eventãcheckpoint/resume facts å?
  unsupported capabilityã?

å æ­¤ï¼SDK readiness çæä½æ åæ¯ï¼?

- å¬å± faÃ§ade åªæ´é?builderãrunnerãrequest/response DTOãevent streamãtyped error å?registry APIã?
- ææ?DTO å¯åºååï¼ææ?runtime handle éè¿ typed port æ³¨å¥ï¼ä¸è¿å¥ wire contractã?
- `northhing-agent-runtime`ãTool primitivesãRuntime Services å?Harness è½éè¿ fake provider ç¬ç«æµè¯ã?
- SDK minimal feature ä¸çµå¼?DesktopãTauriãGit providerãMCP clientãAI HTTP clientãremote SSH æäº§å?UIã?
- å®æ´äº§åè½ååªè½éè¿ Product Assembly æå¼å®?`northhing-core/product-full` ç»è£ï¼ä¸ååæ±¡æ SDK APIã?

åªè¦å¤é¨è°ç¨æ¹ä»å¿é¡»å¯¼å¥ `northhing-core`ãå¯ç?`product-full`ãææ?concrete service managerãè¯»åäº§åå½ä»?
registry æä¾èµå¨å± mutable stateï¼SDK åå¸è¾¹çå°±ä¸æç«ã?

### 1.2 crate åå

```text
northhing-core-types
northhing-events
northhing-runtime-ports
northhing-runtime-services      # typed service bundle / capability availability
tool-contracts              # Cargo package: northhing-agent-tools

tool-execution              # Cargo package: tool-runtime
northhing-agent-runtime         # agent kernel contracts and portable runtime decisions
northhing-harness               # workflow descriptor / provider / registry contracts
northhing-services-core
northhing-services-integrations
northhing-product-domains
northhing-acp
northhing-core
apps/*
```

ç®æ ä¾èµï¼?

```text
apps/*
  -> northhing-core æ?Product Assembly crate
  -> æéä¾èµ northhing-acp / transport / api-layer

Product Assembly
  -> product capability packs
  -> northhing-agent-runtime
  -> northhing-harness
  -> tool-contracts / tool-execution
  -> northhing-runtime-services
  -> adapters / services

Product Capability packs
  -> northhing-harness
  -> northhing-agent-runtime

  -> northhing-product-domains

northhing-agent-runtime
  -> northhing-runtime-ports
  -> northhing-events
  -> northhing-agent-stream
  -> tool-contracts
  -> northhing-runtime-services

tool-execution
  -> tool-contracts
  -> northhing-runtime-ports
  -> northhing-events

northhing-runtime-services
  -> northhing-runtime-ports
  -> northhing-core-types / northhing-eventsï¼ä»å½?service DTO æ?event contract éè¦æ¶å¼å¥ï¼?

adapters / services
  -> northhing-runtime-ports
  -> northhing-core-types
  -> åè®¸ç?third-party ä¾èµ
  -> External Systems
```

ç¦æ­¢ä¾èµï¼?

- `northhing-runtime-ports` -> `northhing-core`
- `tool-contracts` -> å·ä½ service crate
- `tool-execution` -> äº§å registry / permission policy / å·ä½ tool å®ç° crate
- `northhing-agent-runtime` -> `northhing-core`
- `northhing-agent-runtime` -> Tauri / CLI / ACP protocol / Web UI
- `northhing-harness` -> å·ä½ filesystem / Git / terminal manager

ç®æ  crate åå»ºæç»§ç»­æ©å±åå¥ï¼

- åªæå½?owner è¾¹çãæ§è·¯å¾å¼å®¹ãfocused testsãä¾èµæ¶çå boundary check é½è½åæ¶è½å°æ¶ï¼æåå»ºæ°çç®æ ?crateã?
- `northhing-runtime-services` çæ©å±å¿é¡»ä¿æ?typed builderãæ¬å?serviceãremote service å?fake provider ä¸ç±»æ³¨å¥è·¯å¾å¯æµè¯ã?
- `northhing-agent-runtime` çæ©å±å¿é¡»ä¿ææ§è·¯å¾ facadeãfocused tests å?boundary checkï¼ä¸ä¸å¾å¸æ¶ concrete serviceãproduct surface æå¹³å°å®ç°ã?
- `northhing-harness` çæ©å±å¿é¡»ä¿æ?descriptor / registryãæ§è·¯å¾å¼å®¹ãfocused tests å?boundary checkï¼ä¸ä¸å¾æ?provider æ³¨åè¯¯åæ?concrete workflow executionã?
- è¥ç®æ ?crate åªè½æ¿æ¥åä¸ª helper æåªè½éè¿ `northhing-core` æè½æµè¯ï¼åºç»§ç»­çå¨åå§å¼å®¹ facadeï¼ä¸æåæ?crateã?

## 2. ç¨³å®æ¥å£ä¸è¿è¡æ¶æå¡

### 2.1 ç¨³å®å¥çº¦ï¼Stable Contractsï¼?

æå±?crateï¼?

- `northhing-core-types`
- `northhing-events`
- `northhing-runtime-ports`

å»ºè®®æ¨¡åï¼?

```text
northhing-core-types
  error/
  identity/
  artifact/
  usage/
  surface/

northhing-events
  runtime/
  tool/
  permission/
  product/

northhing-runtime-ports
  agent/
  service/
  permission/
  subagent/
  tool/
  workspace/
```

æ¥å£ååï¼?

- DTO å¿é¡»å¯åºååï¼é¿åæºå¸?runtime handleã?
- port trait åªæè¿°è½åï¼ä¸æè¿°äº§å?UIã?
- permission / approval å¿é¡»åå« surfaceãthreadãturnãagentãsubagent identityã?
- artifact ref ä½¿ç¨ç¨³å® URI / logical pathï¼ä¸æ´é²æ¬å°ç»å¯¹è·¯å¾ã?

ç¤ºä¾æ¥å£ï¼?

```rust
pub trait RuntimeEventSink: Send + Sync {
    fn emit(&self, event: RuntimeEvent);
}

#[async_trait::async_trait]
pub trait PermissionPort: Send + Sync {
    async fn request(&self, request: PermissionRequest) -> PermissionDecision;
}

#[async_trait::async_trait]
pub trait WorkspacePort: Send + Sync {
    async fn resolve(&self, identity: WorkspaceIdentity) -> Result<WorkspaceFacts, PortError>;
}
```

### 2.2 Runtime Services

ç®æ  owner crateï¼`northhing-runtime-services`ã?

èè´£ï¼?

- æ¿è½½ runtime å¯æ¶è´¹ç typed service bundleã?
- æä¾ provider æ³¨åå?capability resolutionã?
- æå·ä½å®ç°ä¸ runtime port éç¦»ã?
- æä¾ç»ä¸ç?unavailable / unsupported éè¯¯ã?
- ä¸ºæµè¯æä¾?fake provider builderã?

å»ºè®®åé¨æ¨¡åï¼?

```text
northhing-runtime-services
  bundle.rs             # RuntimeServices / ToolServices / HarnessServices
  builder.rs            # typed builder
  capability.rs         # capability ids ä¸?availability
  registry.rs           # provider æ³¨å
  errors.rs             # unsupported / unavailable æ å°
  test_support.rs       # fake providers
```

æ ¸å¿ç»æï¼?

```rust
pub struct RuntimeServices {
    pub filesystem: Arc<dyn FileSystemPort>,
    pub workspace: Arc<dyn WorkspacePort>,
    pub session_store: Arc<dyn SessionStorePort>,
    pub permission: Arc<dyn PermissionPort>,
    pub events: Arc<dyn RuntimeEventSink>,
    pub clock: Arc<dyn ClockPort>,
    pub terminal: Option<Arc<dyn TerminalPort>>,
    pub network: Option<Arc<dyn NetworkPort>>,
    pub git: Option<Arc<dyn GitPort>>,
    pub mcp_catalog: Option<Arc<dyn McpCatalogPort>>,
    pub remote_connection: Option<Arc<dyn RemoteConnectionPort>>,
    pub remote_workspace: Option<Arc<dyn RemoteWorkspacePort>>,
    pub remote_projection: Option<Arc<dyn RemoteProjectionPort>>,
    pub remote_capabilities: Option<Arc<dyn RemoteCapabilityPort>>,
}

pub struct RuntimeServicesBuilder {
    // ä»?typed å­æ®µ
}

impl RuntimeServicesBuilder {
    pub fn with_filesystem(self, port: Arc<dyn FileSystemPort>) -> Self;
    pub fn with_optional_network(self, port: Option<Arc<dyn NetworkPort>>) -> Self;
    pub fn with_optional_git(self, port: Option<Arc<dyn GitPort>>) -> Self;
    pub fn with_optional_remote_connection(self, port: Option<Arc<dyn RemoteConnectionPort>>) -> Self;
    pub fn with_optional_remote_workspace(self, port: Option<Arc<dyn RemoteWorkspacePort>>) -> Self;
    pub fn with_optional_remote_projection(self, port: Option<Arc<dyn RemoteProjectionPort>>) -> Self;
    pub fn with_optional_remote_capabilities(self, port: Option<Arc<dyn RemoteCapabilityPort>>) -> Self;
    pub fn build(self) -> Result<RuntimeServices, RuntimeServicesError>;
}
```

Remote ports çè¾¹çï¼

- `RemoteConnectionPort` åªæè¿°è¿æ¥èº«ä»½ãç¶æãè®¤è¯ä¸ä¸æåè¿æ¥çå½å¨æè¯·æ±ï¼ä¸æ´é?SSH / relay / tunnel concrete handleã?
- `RemoteWorkspacePort` åªæè¿?remote workspace identityãroot resolutionãstartup guard å?persistence/session factsã?
- `RemoteProjectionPort` åªæè¿?fileãterminalãimage/context projection ç?request / response shapeï¼ä¸ç´æ¥æ§è¡å·ä½ OS å½ä»¤ã?
- `RemoteCapabilityPort` åªæè¿?remote host capability factsï¼ä¾å¦?filesystemãterminalãreview platformãmodel catalog æ¯æç¶æã?
- SSHãrelayãæ¬å°é§éãè¿ç«?OSãè®¤è¯å transport å®ç°å¿é¡»çå¨å·ä½ Remote providerï¼ç± Product Assembly æ³¨åã?

è®¾è®¡çº¦æï¼?

- ä¸æä¾?`get<T>() -> Any` ä½ä¸ºä¸»è·¯å¾ã?
- capability ç¼ºå¤±å¿é¡»è¿å typed unsupported éè¯¯ã?
- ä¸å¨ runtime services ä¸­æ§è¡äº§åå½ä»¤ã?
- ä¸å¨ runtime services ä¸­åå»?concrete managerï¼åå»ºåçå¨ Product Assemblyã?
- `RuntimeServices` æ¯è¿è¡æ¶ä¾èµéåï¼ä¸æ¯å¨å± mutable app stateã?

## 3. Runtime / Tool / Harness åæ ¸

### 3.1 Agent Runtime SDK

ç®æ  owner crateï¼`northhing-agent-runtime`ã?

ç®æ èè´£ï¼?

- session çå½å¨æã?
- dialog turn / model round çå½å¨æã?
- scheduler / queue / cancellationã?
- prompt loop å?context assemblyã?
- prompt cache åè°ã?
- agent definition registryãsubagent registry æ¥è¯¢å?delegation policyã?
- fork context seedingã?
- tool call è°åº¦ã?
- permission åè°ã?
- runtime eventsã?
- post-turn processorã?

å¬å± faÃ§adeï¼?

```rust
pub struct AgentRuntimeBuilder {
    // typed runtime parts only
}

pub struct AgentRunRequest {
    pub session: SessionSelector,
    pub input: AgentInput,
    pub cancellation: CancellationToken,
}

pub struct AgentRunHandle {
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub events: AgentEventStream,
}

impl AgentRuntimeBuilder {
    pub fn with_services(self, services: RuntimeServices) -> Self;
    pub fn with_tools(self, tools: Arc<ToolRuntime>) -> Self;
    pub fn with_harnesses(self, harnesses: Arc<HarnessRegistry>) -> Self;
    pub fn with_agents(self, agents: Arc<dyn AgentDefinitionRegistry>) -> Self;
    pub fn with_hooks(self, hooks: RuntimeHookRegistry) -> Self;
    pub fn build(self) -> Result<AgentRuntime, RuntimeBuildError>;
}

impl AgentRuntime {
    pub async fn run(&self, request: AgentRunRequest) -> Result<AgentRunHandle, RuntimeError>;
}
```

è¯?faÃ§ade æ¯ç®æ ?API å½¢æãå®å¿é¡»åªæ¥æ¶å·²ç»è£ç?typed partsï¼ä¸è´è´£åå»º
filesystemãterminalãMCPãAI clientãRemote provider æäº§åå½ä»¤ã?

æ§è·¯å¾å¼å®¹çº¦æï¼

- `northhing-agent-runtime` åªè½ä¾èµç¨³å®å¥çº¦ãTool RuntimeãRuntime Services æ¥å£åæ³¨å¥ç providerã?
- concrete scheduler çå½å¨æãsession metadata storeãtoken subscriberãevent deliveryãproduct `Tool`
  handlerãconcrete prompt assemblyãworkspace / remote / config IOãcustom subagent file IO åå¹³å?adapter
  å¨è¡ä¸ºç­ä»·æªè¯æåä¸å¾ä¸æ²å° runtime kernelã?
- promptãeventãthread goalãscheduler æ?subagent ççº¯äºå®å¦æè¿å¥ Agent Runtime SDKï¼å¿é¡»åæ¶å é¤æ§ owner
  å®ç°ä¸»ä½ï¼ä¿çæ§è·¯å¾å¼å®¹ï¼å¹¶å·å¤ focused contract test ä¸?boundary checkã?

å»ºè®®åé¨æ¨¡åï¼?

```text
northhing-agent-runtime
  lib.rs
  runtime.rs            # AgentRuntime å¬å± API
  config.rs             # RuntimeConfig
  session/
    manager.rs
    state.rs
    persistence.rs
  turn/
    dialog_turn.rs
    model_round.rs
    continuation.rs
  scheduler/
    queue.rs
    cancellation.rs
    priority.rs
  prompt/
    assembly.rs
    cache.rs
    compression.rs
  agents/
    definitions.rs
    registry.rs
    prompts.rs
  subagent/
    delegation.rs
    fork_context.rs
    background.rs
  tools/
    dispatcher.rs
    permission.rs
    result_bridge.rs
  hooks/
    registry.rs
    prompt.rs
    post_turn.rs
  events/
    mapper.rs
```

å¬å± APIï¼?

```rust
pub struct AgentRuntime {
    services: RuntimeServices,
    tools: Arc<ToolRuntime>,
    agents: Arc<dyn AgentDefinitionRegistry>,
    hooks: Arc<RuntimeHookRegistry>,
    config: RuntimeConfig,
}

impl AgentRuntime {
    pub fn new(parts: AgentRuntimeParts) -> Result<Self, RuntimeBuildError>;

    pub async fn start_session(
        &self,
        request: StartSessionRequest,
    ) -> Result<SessionHandle, RuntimeError>;

    pub async fn submit_turn(
        &self,
        request: SubmitTurnRequest,
    ) -> Result<TurnHandle, RuntimeError>;

    pub async fn cancel_turn(
        &self,
        request: CancelTurnRequest,
    ) -> Result<CancelOutcome, RuntimeError>;
}
```

è¾å¥ï¼?

- `RuntimeServices`
- `ToolRuntime`
- `AgentDefinitionRegistry`
- `RuntimeHookRegistry`
- model / stream adapter
- äº§åæ³¨å¥ç?`RuntimeConfig`

è¾åºï¼?

- `RuntimeEvent`
- transcript delta
- artifact refs
- permission requests
- session state
- turn outcome

ä¸å¾æ¥æï¼?

- å·ä½ filesystem / Git / terminal / MCP clientã?
- TauriãCLI TUIãWeb renderingã?
- ACP protocolã?
- äº§å feature matrixã?
- å·ä½ tool å®ç°ã?

å³é®ä¿æ¤ï¼?

- `SessionManager -> Session -> DialogTurn -> ModelRound` è¯­ä¹ä¸åã?
- `/goal` custom metadataãpost-turn verificationãcontinuation event ä¸æ¼ç§»ã?
- `get_goal` / `create_goal` / `update_goal` ç?tool response wire shapeãblocked/complete è¯­ä¹å?token budget report ä¸æ¼ç§»ã?
- `Task.run_in_background` delivery ä¸æ¼ç§»ã?
- `Task.fork_context` ç¦æ­¢å­æ®µãprompt cache cloneãcontext seeding ä¸æ¼ç§»ã?
- DeepResearch citation renumber post-turn hook ä¿æ deterministicã?

### 3.2 Tool Primitives

æå±?crateï¼?

- `tool-contracts`ï¼Cargo package: `northhing-agent-tools`ï¼?- `tool-execution`ï¼Cargo package: `tool-runtime`ï¼?

ç®æ èè´£ï¼?

- `tool-contracts`ï¼tool DTOãmanifestãexposureãschemaãpath policyãresult policyãadmission gate å?provider-neutral registry assemblyã?- `tool-execution`ï¼ä½å±?file/search/tool IO helperï¼ä¸æ¥æäº§å registryãpermission policy æ?agent-facing tool surfaceã?

å»ºè®®æ¨¡åï¼?

```text
tool-contracts
  framework.rs
  restrictions.rs
  file_guidance.rs
  tool_result_storage.rs
  tool_execution_presentation.rs


tool-execution
  filesystem.rs
  search.rs
  remote.rs
  result_window.rs
```

æ ¸å¿æ¥å£ï¼?

```rust
#[async_trait::async_trait]
pub trait ToolProvider: Send + Sync {
    fn id(&self) -> ToolProviderId;
    fn manifest(&self, ctx: ToolManifestContext) -> ToolManifest;
    async fn get(&self, name: &str) -> Option<Arc<dyn RuntimeTool>>;
}

#[async_trait::async_trait]
pub trait RuntimeTool: Send + Sync {
    fn spec(&self, ctx: ToolSpecContext) -> ToolSpec;

    async fn execute(
        &self,
        ctx: ToolExecutionContext,
        input: ToolInput,
    ) -> Result<ToolExecutionOutput, ToolExecutionError>;
}

pub struct ToolExecutionContext {
    pub facts: ToolContextFacts,
    pub services: ToolExecutionServices,
    pub cancellation: CancellationToken,
}
```

ç®æ èè´£ï¼?

- provider-neutral manifestãcatalogãpermission gateãexecution admissionãtool hookãexecution result
  presentation å?result artifact policyã?
- `GetToolSpec` catalogãdetailãassistant result å?collapsed-tool unlock observationã?
- workspace serviceãpath policyãruntime artifact referenceãremote path containment å?tool context facts ç?
  ç¨³å® contractã?

æ§è·¯å¾å¼å®¹çº¦æï¼

- core å¯ä»¥ä¿çæ§è·¯å¾?facadeãconcrete tool adapterãstate updateãregistry lookupãconfirmationãactual
  execution å?filesystem persistenceï¼ç®æ ç¶æè¦æ±åªæå¨ç­ä»·æµè¯ä¿æ¤ä¸æè½ç§»å¨è¿äºè¡ä¸ºã?
- workspace file/shell contract ä¿çæ¢æéè¯¯ä¸åæ¶è¯­ä¹ï¼ä¸å¾æéè¯¯åç±»ãåæ¶è¯­ä¹æäº§å tool exposure
  åæ´æ··å¥ owner è¾¹çç§»å¨ã?

è®¾è®¡çº¦æï¼?

- `ToolExecutionContext` ä¸æ´é²å·ä½?managerã?
- `ToolContextFacts` åªåå?portable factsã?
- Tool primitives åªæ¶è´?`ToolExecutionServices` è¿æ ·ççª service è§å¾ï¼ä¸ä¾èµå®æ´
  `RuntimeServices` bundleã?
- path policyãruntime artifact refãremote POSIX containment ç?`tool-contracts` æ¿è½½ã?
- MCP tool ä½ä¸º external tool provider æ³¨å¥ï¼ä¸åç½®å?Agent Runtime SDKã?
- `GetToolSpec` æ?tool catalog è½åï¼ä¸æ¯äº§å?UIã?

å¿é¡»ä¿æ¤ï¼?

- prompt-visible manifestã?
- expanded / collapsed exposureã?
- `GetToolSpec` schema / assistant detail / detail JSONã?
- collapsed unlock state ä¸?persistence çå½å¨æã?
- readonly / enabled snapshot filterã?
- MCP / ACP / desktop tool catalog ç­ä»·ã?
- oversized tool result persistenceãflushãpreviewãartifact refã?
- Write/Edit/Read file-read-state guardrailã?

### 3.3 Harness Layer

ç®æ  owner crateï¼`northhing-harness`ã?

èè´£ï¼?

- æ?SDDãDeepReviewãDeepResearchãMiniAppãfunction-agent ç­å·¥ä½æµä»?runtime kernel ä¸­åç¦»ã?
- å®ä¹ workflow descriptorãroute planãprovider registryãworkflow planãstepãpolicyãartifactã?
  review gate å?post-processorã?
- éè¿ Agent Runtime SDKãTool Runtime å?service ports ç¼æã?

å»ºè®®åé¨æ¨¡åï¼?

```text
northhing-harness
  provider.rs
  registry.rs
  plan.rs
  context.rs
  artifact.rs
  hooks.rs
  review_gate.rs
  sdd/
  deep_review/
  deep_research/
  miniapp/
```

æ ¸å¿æ¥å£ï¼?

```rust
#[async_trait::async_trait]
pub trait HarnessProvider: Send + Sync {
    fn id(&self) -> HarnessId;
    fn capabilities(&self) -> HarnessCapabilities;

    async fn plan(
        &self,
        ctx: HarnessPlanningContext,
        input: HarnessInput,
    ) -> Result<HarnessPlan, HarnessError>;

    async fn execute(
        &self,
        ctx: HarnessExecutionContext,
        plan: HarnessPlan,
    ) -> Result<HarnessOutcome, HarnessError>;
}

pub struct HarnessExecutionContext {
    pub runtime: Arc<AgentRuntime>,
    pub tools: Arc<ToolRuntime>,
    pub services: HarnessServices,
    pub events: Arc<dyn RuntimeEventSink>,
}
```

è®¾è®¡çº¦æï¼?

- harness å¯ä»¥ç¼æ runtime/toolï¼ä½ä¸æ¥æ?session manager internalsã?
- harness ä¸ç´æ¥è®¿é?concrete filesystem / Git / terminalã?
- äº§åå½ä»¤åªæ å°å° harness capabilityï¼ä¸æå½ä»¤å±ç¤ºé»è¾ä¸æ²ã?
- æ?harness éè¿ provider æ³¨åï¼ä¸æ?Agent Runtime SDK åæ ¸ã?
- descriptor-only / legacy-facade provider åªè½è¡¨è¾¾ route planï¼ä¸å¾è¢«æè¿°ä¸ºå·²ç»æ¥æ?concrete workflow executionã?
  æ§è¡è¯­ä¹ç§»å¨å¿é¡»åç¬è¯æè¡ä¸ºç­ä»·ã?

## 4. äº§åç»è£ä¸æ©å±?

### 4.1 Product Assembly

Product Assembly æ?composition rootãåå§ç¶æå¯ç?`northhing-core` å¼å®¹ facade æ¿è½½ï¼ç®æ ç¶æå¯ææç¬ç«
Product Assembly crateã?

èè´£ï¼?

- åå»ºææ¥æ¶å·ä½?adapter / service å®ç°ã?
- æå»º `RuntimeServices`ã?
- æ³¨å tool provider groupsã?
- æ³¨å harness providersã?
- æ³¨å agent definitionsãsubagentsãskillsãprompt modulesã?
- å»ºç«äº§å feature matrixã?
- æ?interface å½ä»¤æ å°å?capability / harness / runtime requestã?
- æ ¹æ®äº¤ä»å½¢æéæ© `DeliveryProfile`ã`CapabilitySet`ãadapter å?service provider éåã?
- å¯¹ä¸æ¯æè½åè¿å typed unsupported / unavailable éè¯¯ï¼èä¸æ¯è®©ä¸å± runtime å¤æ­äº§åå½¢æã?

å»ºè®®æ¨¡åï¼?

```text
product-assembly
  full.rs
  delivery_profile.rs
  capability_set.rs
  desktop.rs
  cli.rs
  server.rs
  remote.rs
  acp.rs
  feature_matrix.rs
  commands.rs
```

æ ¸å¿ç»æï¼?

```rust
pub enum DeliveryProfile {
    Desktop,
    Cli,
    Server,
    Remote,
    Acp,
    Web,
}

pub struct CapabilitySet {
    pub agent_modes: Vec<AgentModeId>,
    pub tool_packs: Vec<ToolPackId>,
    pub harness_packs: Vec<HarnessId>,
    pub service_capabilities: Vec<ServiceCapabilityId>,
    pub command_providers: Vec<CommandProviderId>,
}

pub struct ProductAssemblyPlan {
    pub profile: DeliveryProfile,
    pub capabilities: CapabilitySet,
    pub feature_groups: Vec<FeatureGroupId>,
}

pub trait ProductAssembler {
    fn plan(&self, profile: DeliveryProfile) -> Result<ProductAssemblyPlan, AssemblyError>;
    fn build(&self, plan: ProductAssemblyPlan) -> Result<ProductRuntime, AssemblyError>;
}
```

å®ç°æ³¨åæ¹å¼ï¼?

```rust
pub struct ProductAssemblyInput {
    pub profile: DeliveryProfile,
    pub services: ConcreteServiceProviders,
    pub tool_providers: Vec<Arc<dyn ToolProvider>>,
    pub harness_providers: Vec<Arc<dyn HarnessProvider>>,
    pub agents: Arc<dyn AgentDefinitionRegistry>,
    pub commands: Vec<CommandProviderRef>,
    pub hooks: RuntimeHookRegistry,
}

pub struct ProductRuntimeParts {
    pub services: RuntimeServices,
    pub tools: Arc<ToolRuntime>,
    pub harnesses: Arc<HarnessRegistry>,
    pub agents: Arc<dyn AgentDefinitionRegistry>,
    pub commands: ProductCommandRegistry,
    pub hooks: RuntimeHookRegistry,
}
```

æ³¨åè·¯å¾ï¼?

- concrete service provider åªæ³¨åå° `RuntimeServicesBuilder`ã?
- tool provider åªæ³¨åå° `ToolRuntimeBuilder::install_provider`ã?
- harness provider åªæ³¨åå° `HarnessRegistryBuilder`ã?
- agentãsubagentãpromptãskill åªæ³¨åå° `AgentDefinitionRegistry` æå¯¹åº?registryã?
- è¾å¥æ¡å½ä»¤ãå®¡æ ¸å¥å£ãMiniApp å¥å£åªæ³¨åå° `ProductCommandRegistry`ï¼åæ å°å?capability æ?harnessã?
- unsupported / unavailable è½åå?`CapabilityAvailability` ä¸­è¡¨è¾¾ï¼ä¸è®© runtime kernel è¯»åäº§åå½¢æã?

ç¤ºä¾æå»ºæµç¨ï¼?

```rust
pub fn build_desktop_runtime(input: DesktopAssemblyInput) -> Result<ProductRuntime, AssemblyError> {
    let services = RuntimeServicesBuilder::new()
        .with_filesystem(input.desktop_fs)
        .with_workspace(input.workspace)
        .with_permission(input.permission)
        .with_optional_git(input.git)
        .build()?;

    let tools = ToolRuntimeBuilder::new()
        .install_provider(input.core_tools)
        .install_provider(input.mcp_tools)
        .build()?;

    let runtime = AgentRuntime::new(AgentRuntimeParts {
        services,
        tools,
        agents: input.agents,
        hooks: input.runtime_hooks,
        config: input.config,
    })?;

    Ok(ProductRuntime { runtime })
}
```

çº¦æï¼?

- Product Assembly å¯ä»¥ä¾èµå·ä½å®ç°ï¼runtime kernel ä¸å¯ä»¥ã?
- ä¸åäº§åå¯ä»¥æ³¨åä¸å surface commandï¼ä½å¿é¡»æ å°å°ç¨³å®?capabilityã?
- è¾å¥æ¡å½ä»¤ãå®¡æ ¸ãMiniAppãACP clientãèªå®ä¹ tool/subagent/skill åéè¿ assembly æ³¨åã?
- assembly ä¸å¾æ¹ååºå± runtime è¯­ä¹æ¥ééæä¸ª surfaceã?
- `DeliveryProfile` åªè½å½±å capability/provider éæ©ï¼ä¸å¾è®©ä¸å±åºç° `if desktop`
  æ?`if cli` è¿æ ·ç?product åæ¯ã?
- Tauri handleãwindowãcommand macro å?desktop app state åªè½å­å¨äº?Desktop provider æ?
  transport/API adapterï¼runtime parts åªæ¥æ?typed service portãDTOãevent fact å?capability availabilityã?
- feature group æ¯æå»ºæ¶è½åè¾¹çï¼`CapabilitySet` æ¯äº§åè¿è¡æ¶è½åè¾¹çï¼ä¸¤èå¿é¡»å¨
  assembly ä¸­æ¾å¼å¯¹åºã?
- ä»»ä½äº¤ä»å½¢æåå°è½ååï¼å¿é¡»åæ´æ° product matrix å¹¶è¡¥äº§åå¥å£éªè¯ã?

### 4.2 äº§åå½¢æä¸ç»è£å·®å¼

| äº§åå½¢æ?| å³é®å·®å¼ | ç»è£æ¶å¿é¡»ç¨³å®çä¸å±å¥çº¦ |
|---|---|---|
| Desktop | Tauri windowãdesktop APIãæ¬å?permission UI | runtime eventsãpermission factsãartifact refsãdesktop service providers |
| CLI | TUIãå½ä»¤è¾å¥ãç»ç«¯å±ç¤ºãpackage workflow | command providerãagent/session/tool contractãCLI-safe service providers |
| Server | HTTP/WebSocket routeãserver workspace policy | transport DTOãruntime request/responseãworkspace identity |
| Remote / mobile | remote workspaceãrelay/botãfile/terminal projection | remote stateãlogical pathãpermission/event facts |
| ACP | ACP protocolãclient lifecycleãremote probing | external agent/tool capabilityãenvironment facts |
| Web UI / mobile web | UI stateãhydrationãpairingãsession å±ç¤º | API/transport DTOãruntime event facts |

### 4.3 Product Capability è®¾è®¡

Product Capability ä½äº Product Assembly ä¸?Harness / Runtime / Tool ä¹é´ï¼è´è´£æå¤§åäº§åè½å
ææå¯ç»è£ç capability packãå®ä¸æ¥æ?UIï¼ä¹ä¸ç´æ¥æ§è¡å·ä½?IOã?

å»ºè®®æ¨¡åï¼?

```text
product-capabilities
  code_agent.rs
  deep_review.rs
  deep_research.rs
  miniapp.rs
  function_agent.rs
  remote_control.rs
  mcp_app.rs
  computer_use.rs
  command_mapping.rs
```

æ ¸å¿æ¥å£ï¼?

```rust
pub trait CapabilityPack: Send + Sync {
    fn id(&self) -> CapabilityId;
    fn required_services(&self) -> Vec<ServiceCapabilityId>;
    fn tool_packs(&self) -> Vec<ToolPackId>;
    fn harness_packs(&self) -> Vec<HarnessId>;
    fn agent_definitions(&self) -> Vec<AgentDefinitionRef>;
    fn command_providers(&self) -> Vec<CommandProviderRef>;
}
```

åå±è§åï¼?

- Code Agent pack å¯ä»¥å£°æ agent modesãtool packsãprompt modulesï¼ä½ä¸æ¥æ?tool executionã?
- Deep Review pack å¯ä»¥å£°æ harness providerãreport artifact contractãqueue/retry policyï¼?
  ä½?target resolution å?UI construction çå¨ surfaceã?
- MiniApp pack å¯ä»¥å£°æ MiniApp harnessãdomain portsãartifact policyï¼ä½ worker process å?
  filesystem IO éè¿ Runtime Services providerã?
- MCP App pack å¯ä»¥å£°æ MCP tool/resource/prompt capabilityï¼ä½ MCP transport å±äº
  `northhing-services-integrations`ã?
- Input command pack åªå£°æ?command å?capability/harness/runtime request çæ å°ï¼ä¸å±äº«å·ä½?UIã?

### 4.4 ACP æ©å±æ¹å¼

`northhing-acp` ä¿æ integration ownerã?

ç»§ç»­æ¥æï¼?

- ACP protocolã?
- ACP client lifecycleã?
- config persistenceã?
- remote probingã?
- startup timeoutã?
- workspace surface selectionã?

åä¸æ´é²ï¼?

```rust
pub trait ExternalAgentProvider: Send + Sync {
    fn list_agents(&self) -> Vec<ExternalAgentDescriptor>;
    async fn start(&self, request: ExternalAgentStartRequest) -> Result<ExternalAgentSession, AcpError>;
}

pub trait ExternalToolProvider: Send + Sync {
    fn tool_manifest(&self, ctx: ToolManifestContext) -> ToolManifest;
}
```

Agent Runtime SDK åªè½çå° external agent/tool capabilityï¼ä¸æç¥ ACP protocolãè¿ç¨ç®¡çã?
remote probing æ?startup timeoutã?

### 4.5 Skills / Prompt / Subagent

å»ºè®®å½å±ï¼?

- prompt moduleï¼Agent Runtime SDK ç?prompt assembly contractã?
- skillï¼prompt / resource / instruction æ©å±ï¼ä½ä¸?agent definition æ?harness input çä¸é¨åã?
- subagent definitionï¼Agent Definition Registryã?
- subagent executionï¼Agent Runtime SDKã?
- Task toolï¼Tool Runtime entrypointï¼è°ç?Agent Runtime SDKã?

çº¦æï¼?

- skills ä¸ç´æ¥æäº?service handleã?
- subagent permission æ¥æºå¿é¡»åå« parent sessionãparent agentãtarget agentãsurfaceã?
- prompt module åªå£°æå¯ç»ååå®¹ï¼ä¸æ§è¡ IOã?
- skill resource è®¿é®éè¿ filesystem/workspace portã?

### 4.6 Hook ä¸?Event è®¾è®¡

äºä»¶ï¼?

```rust
pub enum RuntimeEvent {
    SessionStarted(SessionStarted),
    TurnStarted(TurnStarted),
    PromptAssembled(PromptAssembled),
    ToolCallStarted(ToolCallStarted),
    PermissionRequested(PermissionRequested),
    SubagentSpawned(SubagentSpawned),
    ArtifactWritten(ArtifactWritten),
    TurnCompleted(TurnCompleted),
}
```

Runtime hookï¼?

```rust
#[async_trait::async_trait]
pub trait PromptDecorator: Send + Sync {
    async fn decorate(&self, ctx: PromptHookContext, prompt: PromptBundle)
        -> Result<PromptBundle, HookError>;
}

#[async_trait::async_trait]
pub trait PostTurnProcessor: Send + Sync {
    async fn process(&self, ctx: PostTurnContext, outcome: TurnOutcome)
        -> Result<TurnOutcome, HookError>;
}
```

Tool hookï¼?

```rust
#[async_trait::async_trait]
pub trait BeforeToolExecution: Send + Sync {
    async fn before(&self, ctx: ToolExecutionContext, input: ToolInput)
        -> Result<ToolInput, HookError>;
}
```

è§åï¼?

- hook registry å¿é¡»æç¨³å®é¡ºåºã?
- hook å¿é¡»æ?timeoutã?
- hook error å¿é¡»å¯åç±»ï¼fail turnãskip hookãdeny toolãrecord warningã?
- hook ä¸å¾è·åæªå£°æçå·ä½ serviceã?
- ä¿®æ¹ prompt / manifest / output ç?hook å¿é¡»æ?snapshot æµè¯ã?

## 5. è´¨éä¿æ¤ä¸ç®æ æå¤å®?

### 5.1 é²æ£æ§è®¾è®?

éè¯¯ï¼?

- contract å±ä½¿ç?portable error factsã?
- Agent Runtime SDK / Runtime Services è´è´£éè¯¯åç±»åäºä»¶ä¸æ¥è¾¹çã?
- Product Surface åªè´è´£å±ç¤ºé»è¾ã?
- unsupported capability å¿é¡»æç¡®ï¼ä¸åè®¸æ³åä¸?unknown failureã?

åæ¶ï¼?

- turnãtoolãsubagentãharness step é½å¿é¡»æ¥æ?cancellationã?
- cancellation outcome å¿é¡»å¯è§æµã?
- background task å¿é¡»æ?result delivery æ?explicit detached stateã?

æä¹åï¼

- session persistence éè¿ portã?
- artifact write éè¿ portã?
- oversized tool result å¿é¡» flush ååè¿å refã?
- remote/local workspace path éè¿ logical identity è¡¨è¾¾ã?

å¹¶åï¼?

- scheduler queueãsubagent backgroundãfork context å¿é¡»å®ä¹å¹¶åéå¶ã?
- fork context ç»§ç»­ä¿çç¦æ­¢å­æ®µåéå½ subagent ä¿æ¤ã?
- provider registry æå»ºååºå°½é immutableï¼é¿å?runtime æé´ materialization æ¼ç§»ã?

### 5.2 è®¾è®¡è¾¹ç

æ¬æåªæè¿°ç®æ æ¥å£ãcrate åé¨ç»æåè¡ä¸ºä¿æ¤è¦æ±ãè¥éªè¯åç°ç®æ æ¥å£ãcrate å½å±ãè¡ä¸ºè¾¹çæé£é©å¤æ­ä¸æç«ï¼
åºåä¿®æ­£è®¾è®¡å¤æ­ï¼åè°æ´å®ç°è¾¹çã?

### 5.3 æµè¯ç­ç¥

Contract æµè¯ï¼?

- DTO serialization round-tripã?
- permission facts source identityã?
- artifact ref logical pathã?
- unsupported capability errorã?

Tool æµè¯ï¼?

- manifest orderingã?
- expanded / collapsed exposureã?
- `GetToolSpec` detailã?
- readonly / enabled filterã?
- oversized result persistenceã?

Runtime æµè¯ï¼?

- session start / turn submit / cancelã?
- prompt assembly snapshotã?
- post-turn processor deterministic outputã?
- subagent delegation policyã?
- fork context seedingã?
- background result deliveryã?

Harness æµè¯ï¼?

- provider æ³¨åã?
- plan ç»æã?
- artifact è¾åºã?
- review gateã?
- hook orderã?

Product æµè¯ï¼?

- Desktop / CLI / ACP product checkã?
- Remote workspace è¡ä¸ºã?
- MCP dynamic tool catalogã?
- MiniApp ä¸?review workflowã?

### 5.4 ç®æ æå¤å®å£å¾?

- `northhing-agent-runtime` è½å¨ä¸ä¾èµ?`northhing-core` çæåµä¸æå»º runtime kernelã?
- Agent Runtime SDK faÃ§ade è½éè¿ fake model providerãfake runtime servicesãfake tool provider å?fake
  harness provider å®ææå°?session / turn / event stream æµç¨ã?
- `northhing-runtime-services` æä¾ typed service injectionï¼å¹¶ç?boundary check ä¿æ¤ã?
- `tool-contracts` å?`tool-execution` åå«æ¿æ tool contract åä½å±?execution helperï¼å·ä½?tool éè¿ Product Assembly æ³¨åã?
- `northhing-harness` æ¯æå·¥ä½æµ?provider æ©å±ã?
- `northhing-core` åªä½ä¸ºå¼å®?facade / product-full assemblyã?
- ææäº§åå½¢æéè¿ Product Assembly æ¾å¼å¯ç¨è½åã?
- ææé«é£é©è¡ä¸ºæ?snapshotãfocused regression æ?product check ä¿æ¤ã?
