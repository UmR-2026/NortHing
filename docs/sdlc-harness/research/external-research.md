# northhing å¯éç½®å¼åä½éªä¸å·¥ç¨æ²»çå¤é¨è°ç 

> èå´ï¼å´ç»?AI ç¼ç æºè½ä½ãä»åºæä»¤ãæéä¸æ²ç®±ãå¯éä»£ç å®¡æ¥ãhook/pluginãäº¤ä»ç©å¾è°±ãè´¨éæ²»çåè¯æµä½ç³»æ´çå¤é¨äº§åãè®ºæãæ åä¸è¶å¿ä¿¡å·ã?
> ç¨éï¼ä½ä¸ºäº§åéæ±åæ¶æè®¾è®¡çå¤é¨è¯æ®æ± ãäº§åéæ±ææ¡£åªæç¼å¿è¦äº§åå¤æ­ï¼æ¬æä¿çè¾å®æ´åèèµæã?

## 1. äº§åè¶å¿

| äº§å/æ¹å | æ ¸å¿è½å | å¯?northhing çå¯å?|
|---|---|---|
| [OpenAI Codex](https://openai.com/index/introducing-codex/) / [Codex Cloud](https://developers.openai.com/codex/cloud) / [Codex CLI](https://developers.openai.com/codex/cli) | äºç«¯ä»»å¡ãCLIãæ¬å?äºç«¯æ§è¡ãAGENTS.mdãæ²ç®±ãå®¡æ¹ãæ¥å¿?æµè¯è¯æ® | ç¨æ·ä½éªåºåå´ç»ä»»å¡ãè®¡åãdiff åæ¹åå±å¼ï¼æ§è¡å®å¨ä¸è´¨éæ²»çéè¦æå¼ |
| [Codex approvals/security](https://developers.openai.com/codex/northhingrovals-security) / [Codex hooks](https://developers.openai.com/codex/hooks) | å®¡æ¹æ¨¡å¼ãæ²ç®±ãå¯ä¿¡å½ä»¤ãhook çå½å¨æãä¿¡ä»»å®¡æ?| å®å¨è¾¹çç¬ç«å¸¸é©»ï¼hook æä¸»å¨æ§è¡é¢ç®¡çä¿¡ä»»ç¶æ?|
| [GitHub Copilot ç¼ç æºè½ä½](https://docs.github.com/en/copilot/concepts/agents/cloud-agent/about-cloud-agent) | issue å?PRãActions åå°æ§è¡ãPR å®¡æ¥ãæºè½ä½ä¼è¯ | å¼æ­¥æºè½ä½çæ ¸å¿ä½éªå´ç»ä»»å¡ãè®¡åãåæ´åå®¡æ¥ç»ç» |
| [GitHub Copilot ä»åºæä»¤](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions) | æ¯æä»åºæä»¤ãè·¯å¾æä»¤ãAGENTS.md | é¡¹ç®è§ååºä¼åè¯»åç°æèµäº§ï¼å¹¶æè·¯å¾åä¸ä¸ææ¸è¿å è½½ |
| [GitHub Copilot ä»£ç å®¡æ¥](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/request-a-code-review/use-code-review) | AI å®¡æ¥æä¾è¯è®ºåå»ºè®?| AI å®¡æ¥é»è®¤åºæ¯ä½æ©æ¦å»ºè®®æï¼ä¸å¤©ç¶ç­åé»æ­å®¡æ?|
| [Claude Code](https://github.com/anthropics/claude-code) / [æé](https://code.claude.com/docs/en/permissions) / [sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) | ç»ç«¯ Agentãæééç½®ãæ²ç®±ãallow/deny è§å | äº§åéè¦ç¨ææ¯éç¦»åå°å¼¹çªï¼åæ¶ä¿çç¨æ·å¯çè§£çæ¾è¡è·¯å¾ |
| [CodeRabbit configuration](https://docs.coderabbit.ai/reference/configuration) / [è·¯å¾æä»¤](https://docs.coderabbit.ai/configuration/path-instructions) | å®¡æ¥å¼ºåº¦ãè·¯å¾çº§æä»¤ãè§å?| å®¡æ¥å¼ºåº¦åè·¯å¾è§åå¯éç½®ï¼é»è®¤æ¨¡å¼éä½åªé?|
| [GitLab Duo èªå®ä¹æä»¤](https://docs.gitlab.com/user/gitlab_duo/customize_duo/review_instructions/) / [è­¦åæ¨¡å¼](https://docs.gitlab.com/user/application_security/policies/merge_request_approval_policies/) | å®¡æ¥æä»¤ãå®¡æ¹ç­ç¥ãè­¦åæ¨¡å¼?| å¼ºç­ç¥åºåå»ºè®®æ?è­¦åæ ¡åï¼åè¿å¥å¼ºå¶è¦æ±/é»æ­ |
| [Kiro Specs](https://kiro.dev/docs/specs/) / [Steering](https://kiro.dev/docs/steering/) / [Hooks](https://kiro.dev/docs/hooks/) | spec é©±å¨å¼åãå·¥ä½åº/å¨å±/å¢é steeringãå è½½èå´æ¨¡å¼ãæºè½ä½ hooks | é¡¹ç®ç¥è¯éè¦ä½ç¨åãä¼åçº§åå è½½æ¶æºï¼å¤æä¸ä¸æææ¡ä»¶å è½½ |
| [Jules](https://jules.google/) | éæ©ä»åº/åæ¯ãäºç«¯è®¡åãdiffãç¨æ·æ¹å?| å¼æ­¥ç¼ç æºè½ä½çé«ä½éªå¥å£æ¯è®¡åå?diff å®¡æ¹ |
| [Atlassian Software Collection](https://www.atlassian.com/collections/software) / [Rovo Dev](https://www.atlassian.com/software/rovo-dev) | JiraãConfluenceãBitbucketãPipelinesãPR å®¡æ¥ãacceptance criteria æ£æ?| å¤æé¡¹ç®éè¦è¿æ¥ä»»å¡ãææ¡£ãä»£ç ãCI åå¢éä¸ä¸æï¼ä½åºæéæ¾é² |
| [Harness](https://www.harness.io/) / [Harness AI](https://developer.harness.io/docs/platform/harness-ai/overview) / [Software Delivery Knowledge Graph](https://www.harness.io/blog/knowledge-graphs-for-ai-software-delivery) | CI/CDãæµè¯ãAppSecãSREãææ¬ä¼åãè½¯ä»¶äº¤ä»ç¥è¯å¾è°?| ç¥è¯å¾è°±åºä»æå°é«ä»·å¼åºæ¯å¼å§ï¼ä¿ææ°é²åº¦åå¯éªè¯ä»·å?|
| [OpenCode Plugins](https://opencode.ai/docs/plugins/) / [SDK](https://opencode.ai/docs/sdk/) / [Server API](https://opencode.ai/docs/server/) | JS/TS pluginãhookãèªå®ä¹å·¥å·ãSSE äºä»¶æµ?| å¯æä¾å¼å®¹å±ï¼ä½åºå±å¿é¡»ç?northhing èªå·±çæéãç­ç¥åäºä»¶æ¨¡åçº¦æ |
| [Cursor Bugbot](https://cursor.com/blog/building-bugbot) / [Qodo Code Review](https://docs.qodo.ai/code-review) | PR çº§é»è¾ç¼ºé·ãå®å¨ãåè§å®¡æ?| PR å®¡æ¥æ¯å¢éåé«é£é©åºæ¯çéè¦æ©å± |
| [LangChain Harness Engineering](https://www.langchain.com/blog/improving-deep-agents-with-harness-engineering) | åºå®æ¨¡åä¸ä¼åæºè½ä½å¤é¨å·¥ç¨å±æ¾èæååºåæµè¯è¡¨ç?| promptãä¸ä¸æãå·¥å·ãç­ç¥åå·¥ä½æµæ¯è½åæ æï¼éè¦è¯æµå A/B |

## 2. ç ç©¶ååºåè¶å?

| ç ç©¶/åºå | ä¿¡å· | è®¾è®¡å¯å |
|---|---|---|
| [SWE-bench](https://github.com/swe-bench/SWE-bench) | çå® GitHub issue æ­£æä¸ºä»£ç ?Agent è¯æµåºç¡ | northhing éè¦çå®?issue é»ééåé¿æåå½é?|
| [SWE-Bench Pro](https://labs.scale.com/leaderboard/swe_bench_pro_public) | æ´é¿ç¨ãæ´çå®ãæ´å¤æä»£ç åºæ´é²è¯æµéæ³æ¼ãä»»å¡å¤æ ·æ§åæµè¯å¯é æ§é®é¢?| å¬å¼æ¦åãåé¨ä¿çéãå¤æé¡¹ç®åç¯å¢å¯å¤ç°æ§éè¦ç»åè¯ä¼?|
| [SWE-agent](https://arxiv.org/abs/2405.15793) | æºè½ä½?è®¡ç®æºæ¥å£å½±åä¿®å¤è½å?| å·¥å·ç»æãç»ç«¯åé¦ãéè¯¯åç°åæä»¶æµè§æ¬èº«æ¯è½åæ æ?|
| [Agentless](https://arxiv.org/abs/2407.01489) | ç®åãå¯è§£éçå®ä½?ä¿®å¤/éªè¯æµç¨å¯è¾¾å°å¼ºåºçº¿ | ä¸å®é»è®¤éç¨å¨èªæ²»æå¼ºæµç¨ï¼ç»æåæµç¨åºä½ä¸ºåºçº¿ |
| [Agentic AI in the SDLC](https://arxiv.org/abs/2604.26275) | Agentic SDLC éè¦ä»æ¶æãè¯æ®ãçäº§ååæ²»çåæ¶è¯ä¼?| northhing å¯æ©å±å° SDLCï¼ä½å¿é¡»ä»¥äº§åä½éªåå¯éªè¯ä»·å¼éæ­¥æ¨è¿ |
| [Terminal-Bench](https://arxiv.org/abs/2601.11868) / [Terminal-Bench 3.0](https://www.tbench.ai/) | çå®ç»ç«¯ä»»å¡è¦çè½¯ä»¶å·¥ç¨ãMLãå®å¨ãæ°æ®ç§å­¦ç­åºæ¯ | éè¦ç»ç«¯ä»»å¡åæ¾åå·¥å·è½¨è¿¹è¯æµï¼é²æ­¢ä»»å¡æ³æ¼ååºåæµè¯è¿æå?|
| [RovoDev Code Reviewer](https://arxiv.org/html/2601.01129v1) | å¨çº¿è¯ä¼°æ¾ç¤º AI å®¡æ¥å¯ç¼©ç?PR å¨æï¼ä½ç¼ºå°ä¸ä¸æä¼äº§çéè¯¯åé¦ | æ·±åº¦å®¡æ¥å¿é¡»æä¸ä¸æå®æ´æ§ãé®é¢çå½å¨æãåè¯åé¢ç®æ§å¶ |
| [TraceLLM](https://arxiv.org/html/2602.01253v1) / [LLM-driven requirements change impact analysis](https://arxiv.org/html/2511.00262v1) | LLM å¯è¾å©éæ±è¿½è¸ªååæ´å½±ååæï¼ä½è¾åºä»éææ¬ãå¬åãç²¾åº¦åäººå·¥ç¡®è®¤çº¦æ | éæ±åæ´å½±åé¢åæåºè¾åºåééåãç½®ä¿¡åº¦åäººå·¥æ£æ¥ææ?|
| [Testing with AI Agents](https://arxiv.org/abs/2603.13724) | AI å·²å¤§éåä¸æµè¯çæï¼ä½æµè¯è´¨ééè¦ç»æåè¡¡é | æµè¯è´¨éä¿æ¤è¦å³æ³¨è´¨éãç¨³å®æ§ååå¼æä¼¤ï¼èéä»å¢å æµè¯æ°é?|
| [NIST SP 800-218A](https://csrc.nist.gov/pubs/sp/800/218/a/final) | å°çæå¼ AI ååºç¡æ¨¡åçº³å¥ SSDF çå½å¨æå®è·µ | AI åä¸å¼ååï¼å®å¨å¼åæ¡æ¶éè¦è¦çæ¨¡åãå·¥å·ãæ°æ®ãæéåä¾åºé¾é£é?|

## 3. æ åä¸æ²»çè¶å?

| æ å/æ¹å | ä¿¡å· | è®¾è®¡å¯å |
|---|---|---|
| [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/concepts/semantic-conventions/) | è½¨è¿¹ãææ ãæ¥å¿ãç»ååèµæºéè¦ç»ä¸è¯­ä¹å½å | è´¨éæ°æ®é¢åºå®ä¹ç¨³å®è¯­ä¹å±æ§ï¼é¿åæ¯ä¸ªæ¨¡åèªé äºä»¶å­æ®?|
| [CDEvents](https://cdevents.dev/docs/primer/) | CI/CD äºä»¶å¼ºè°å£°æå¼ãæ¾è¦åãè·¨å·¥å·äºæä½?| northhing çå½å¨æäºä»¶åºä¿æè§èäºå®åæ¾è¦åäºæä½?|
| [SLSA Provenance](https://slsa.dev/spec/v0.1/provenance) / [in-toto](https://slsa.dev/blog/2023/05/in-toto-and-slsa) | æå»ºåä¾åºé¾è¯æ®éè¦è¯´ææ¥æºãæ¶é´åçææ¹å¼ | è¯æ®ååºæ¯ææ¥æº/è¯æå¼ç¨ï¼ä¸ºåå¸å°±ç»ªåº¦åå®¡è®¡é¢çæ¥å£ |
| [OWASP LLM Top 10](https://owasp.org/www-project-top-10-for-large-language-model-applications/) | prompt æ³¨å¥ãææä¿¡æ¯ãä¾åºé¾ãè¿åº¦ä»£çåæ¨¡åæç»æå¡é½æ¯ LLM åºç¨é£é© | Hook/Eventãpluginãå·¥å·ãmemoryãå¤é¨ééå¨å¿é¡»é»è®¤æå°æéãè±æãè¶æ¶åé¢ç® |
| DORA / SPACE / DevEx | éåº¦ãç¨³å®æ§ãåä½åå¼åèä½éªéè¦èåè¡¡é?| ææ ä½ç³»åæ¶è¦çéåº¦ãææ­ãä¿¡å¿ãå®å¨ãè´¨éåææ¬ |
| AI ç¼ç ææ¬æ²»ç | é«çº§æ¨¡åãAI å®¡æ¥ãCI/Actions èµæºåé¿ä¸ä¸æé½ä¼å½¢ææ¾æ§ææ?| æ·±åº¦å®¡æ¥ãè¯æµãHook åæºè½ä½è¿è¡å¿é¡»å°?tokenãèæ¶ãç¼å­å½ä¸­åéçº§åå ä½ä¸ºæ ¸å¿ææ  |

## 4. å¯¹ææ§å®¡æ¥åçè¶å¿å¤æ?

å¤é¨è¶å¿å±åæåå­ç¹ï¼?

1. é»è®¤ä½éªæ­£å¨èµ°åå¿«éæ§è¡ãè®¡åãdiffãæ¹ååè½»éå®¡æ¥ã?
2. é¡¹ç®ç¥è¯æ­£å¨äº§ååä¸ºä»åº/è·¯å¾/å¢éæä»¤ãsteeringãAGENTS.mdãhook å?pluginï¼ä½è¿äºä¸»å¨éç½®å¿é¡»ç»è¿ä¿¡ä»»åæéè¾¹çã?
3. AI å®¡æ¥åé¨ç¦æä»·å¼ï¼ä½åè¿äº§åæ®éæä¾å®¡æ¥å¼ºåº¦ãè¯è®?å»ºè®®æãè­¦åæ¨¡å¼æå¼ºå¶è¦æ±/é»æ­åçº§ã?
4. å®å¨ä¸è´¨éå¿é¡»åå±ï¼prompt æ³¨å¥ãç½ç»ãå­æ®ãMCPãhookãshellãè·¨ç®å½ååå é¤é£é©å¨å¿«éè·¯å¾ä¸­ä¹éè¦æç¡®ææåå¯å®¡è®¡è®°å½ã?
5. å¤æé¡¹ç®è½åä»ç¶éè¦ï¼ä½å¾è°±ãè¯æ®åãéæ±å½±åååå¸å°±ç»ªåº¦åºä½ä¸ºæéæ¾é²çåå°è½åã?
6. åºåæµè¯åæ°æ æ³ç´æ¥è¯æäº§åè´¨éï¼çå®é¡¹ç®çä¿çéãè½¨è¿¹åæ¾ãå¤å®æ åãææ¬ãå®å¨äºä»¶åç¨æ·ææ­ææ ææ¯å¯æ¼è¿è½åçæ ¸å¿è¯ä¼°èµäº§ã?

## 5. åèèµæ?

- OpenAI: [Codex](https://openai.com/index/introducing-codex/), [Codex æºè½ä½?loop](https://openai.com/index/unrolling-the-codex-agent-loop/), [Codex approvals/security](https://developers.openai.com/codex/northhingrovals-security), [Codex sandboxing](https://developers.openai.com/codex/concepts/sandboxing), [Codex hooks](https://developers.openai.com/codex/hooks), [Agent improvement loop](https://developers.openai.com/cookbook/examples/agents_sdk/agent_improvement_loop)
- GitHub: [Copilot ç¼ç æºè½ä½](https://docs.github.com/en/copilot/concepts/agents/cloud-agent/about-cloud-agent), [Copilot ä»åºæä»¤](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions), [Copilot ä»£ç å®¡æ¥](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/request-a-code-review/use-code-review)
- Anthropic: [Claude Code](https://github.com/anthropics/claude-code), [Claude æé](https://code.claude.com/docs/en/permissions), [Claude Code Review](https://code.claude.com/docs/en/code-review), [Claude Code hooks](https://code.claude.com/docs/en/hooks), [Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)
- CodeRabbit ä¸?GitLab: [CodeRabbit configuration](https://docs.coderabbit.ai/reference/configuration), [CodeRabbit è·¯å¾æä»¤](https://docs.coderabbit.ai/configuration/path-instructions), [GitLab Duo èªå®ä¹æä»¤](https://docs.gitlab.com/user/gitlab_duo/customize_duo/review_instructions/), [GitLab å®¡æ¹ç­ç¥](https://docs.gitlab.com/user/application_security/policies/merge_request_approval_policies/)
- Atlassian: [Software Collection](https://www.atlassian.com/collections/software), [Rovo Dev](https://www.atlassian.com/software/rovo-dev), [Acceptance criteria æ£æ¥](https://support.atlassian.com/rovo/docs/check-acceptance-criteria-in-a-code-review/), [RovoDev Code Reviewer paper](https://arxiv.org/html/2601.01129v1)
- Linear ä¸?Jules: [Linear](https://linear.app/), [Jules](https://jules.google/)
- Harness: [AI software delivery platform](https://www.harness.io/), [Harness AI overview](https://developer.harness.io/docs/platform/harness-ai/overview), [Software Delivery Knowledge Graph](https://www.harness.io/blog/knowledge-graphs-for-ai-software-delivery)
- OpenCode ä¸?Kiro: [OpenCode Plugins](https://opencode.ai/docs/plugins/), [OpenCode SDK](https://opencode.ai/docs/sdk/), [OpenCode Server API](https://opencode.ai/docs/server/), [Kiro Specs](https://kiro.dev/docs/specs/), [Kiro Hooks](https://kiro.dev/docs/hooks/), [Kiro Steering](https://kiro.dev/docs/steering/)
- PR å®¡æ¥ç³»ç»: [Cursor Bugbot](https://cursor.com/blog/building-bugbot), [Qodo Code Review](https://docs.qodo.ai/code-review)
- æ åä¸ææ ? [DORA](https://dora.dev/), [SPACE](https://queue.acm.org/detail.cfm?id=3454124), [DevEx](https://queue.acm.org/detail.cfm?id=3595878), [OpenTelemetry semantic conventions](https://opentelemetry.io/docs/concepts/semantic-conventions/), [CDEvents](https://cdevents.dev/docs/primer/), [SLSA provenance](https://slsa.dev/spec/v0.1/provenance), [in-toto and SLSA](https://slsa.dev/blog/2023/05/in-toto-and-slsa), [OWASP LLM Top 10](https://owasp.org/www-project-top-10-for-large-language-model-applications/), [NIST SP 800-218A](https://csrc.nist.gov/pubs/sp/800/218/a/final)
- ç ç©¶: [SWE-bench](https://github.com/swe-bench/SWE-bench), [SWE-Bench Pro](https://labs.scale.com/leaderboard/swe_bench_pro_public), [SWE-agent](https://arxiv.org/abs/2405.15793), [Agentless](https://arxiv.org/abs/2407.01489), [Agentic AI in the SDLC](https://arxiv.org/abs/2604.26275), [Terminal-Bench](https://arxiv.org/abs/2601.11868), [Testing with AI Agents](https://arxiv.org/abs/2603.13724), [TraceLLM](https://arxiv.org/html/2602.01253v1), [LLM-driven requirements change impact analysis](https://arxiv.org/html/2511.00262v1)
