> **v0.1.0 status note**: This document has encoding issues (GBK/UTF-8 mojibake) and references src/web-ui/ which is [missing] in v0.1.0. Preserved for historical reference only.

# northhing Core 忙聥聠猫搂拢忙聻露忙聻聞

忙聹卢忙聳聡忙娄聜忙聥卢 northhing core runtime 忙聥聠猫搂拢莽職聞盲赂陇盲赂陋莽篓鲁氓庐職猫庐戮猫庐隆莽禄麓氓潞娄茂录職**氓聢聺氓搂聥莽聤露忙聙?*氓聮?*莽聸庐忙聽聡莽聤露忙聙?*茫聙?
氓聢聺氓搂聥莽聤露忙聙聛忙聫聫猫驴掳猫庐戮猫庐隆氓禄潞莽芦聥忙聴露莽職聞盲潞聥氓庐聻忙聻露忙聻聞茫聙聛猫聙娄氓聬聢氓聟鲁莽鲁禄氓聮聦盲赂禄猫娄聛茅聴庐茅垄聵茂录聸莽聸庐忙聽聡莽聤露忙聙聛忙聫聫猫驴掳忙聹聼忙聹聸氓聢聠氓卤聜茫聙聛莽篓鲁氓庐職忙聨楼氓聫拢茫聙?
氓庐聻莽聨掳氓陆聮氓卤聻茫聙聛莽禄聞猫拢聟猫戮鹿莽聲聦茫聙聛盲戮聺猫碌聳忙聳鹿氓聬聭氓聮聦茅拢聨茅聶漏莽潞娄忙聺聼茫聙?

忙聹卢忙聳聡猫聛職莽聞娄猫庐戮猫庐隆莽禄聯猫庐潞茫聙聜猫炉娄莽禄聠忙聨楼氓聫拢茫聙聛crate 氓聠聟茅聝篓忙篓隆氓聺聴氓聮聦忙碌聥猫炉聲猫庐戮猫庐隆猫搂聛
[`agent-runtime-services-design.md`](agent-runtime-services-design.md)茫聙?

## 1. 猫聝聦忙聶炉盲赂聨莽聸庐忙聽?

猫庐戮猫庐隆氓禄潞莽芦聥忙聴露茂录聦northhing 氓路虏莽禄聫盲禄?`northhing-core` 盲赂颅忙聤陆氓聡潞盲潞聠猫聥楼氓鹿虏 owner crate茂录聦盲陆聠 `northhing-core` 盲禄聧忙聣驴忙聥聟氓聟录氓庐?facade茫聙?
氓庐聦忙聲麓盲潞搂氓聯聛 runtime 莽禄聞猫拢聟茫聙聛agent loop茫聙聛service 忙聨楼莽潞驴茫聙聛tool materialization 氓聮聦茅聝篓氓聢?product domain
adapter茫聙聜猫驴聶盲赂陋氓陆垄忙聙聛氓聹篓氓聤聼猫聝陆盲赂聤氓聫炉猫驴聬猫隆聦茂录聦盲陆聠盲录職猫庐漏 runtime 忙聥聠猫搂拢忙聦聛莽禄颅茅聺垄盲赂麓盲赂聣盲赂陋茅聴庐茅垄聵茂录?

- 盲潞搂氓聯聛茅聙禄猫戮聭茫聙聛氓鹿鲁氓聫掳忙聨楼氓聟楼氓聮聦氓聟路盲陆聯 service 氓庐聻莽聨掳猫戮鹿莽聲聦盲赂聧氓陇聼莽篓鲁氓庐職茫聙?
- Desktop茫聙聛CLI茫聙聛Server茫聙聛Remote茫聙聛ACP茫聙聛Web 莽颅聣盲潞搂氓聯聛氓陆垄忙聙聛氓庐鹿忙聵聯猫垄芦氓庐聦忙聲麓 `northhing-core` 莽聣碌氓录聲茫聙?
- Tool茫聙聛MCP茫聙聛ACP茫聙聛subagent茫聙聛skills茫聙聛harness 莽颅聣忙聣漏氓卤聲莽聜鹿莽录潞氓掳聭莽禄聼盲赂聙莽職聞氓聢聠氓卤聜氓陆聮氓卤聻茫聙?

莽聸庐忙聽聡氓陆垄忙聙聛盲赂聧忙聵炉氓聹篓 `northhing-core` 氓聠聟莽禄搂莽禄颅忙聣漏氓录聽氓庐聦忙聲?`AgentRuntime`茂录聦猫聙聦忙聵炉氓陆垄忙聢聬氓聫炉莽聥卢莽芦聥氓碌聦氓聟楼莽職聞
Agent Runtime SDK茫聙聜莽篓鲁氓庐職氓楼聭莽潞娄氓庐職盲鹿聣盲赂聤氓卤聜氓聫炉盲戮聺猫碌聳莽職聞忙聨楼氓聫拢茂录聦Product Assembly 猫麓聼猫麓拢忙鲁篓氓聠聦氓聟路盲陆聯氓庐聻莽聨掳茂录?
Runtime Services茫聙聛Tool primitives 氓聮?Harness Layer 氓聢聠氓聢芦茅職聰莽娄禄 service茫聙聛tool茫聙聛氓路楼盲陆聹忙碌聛氓聮聦盲潞搂氓聯聛氓陆垄忙聙聛氓路庐氓录聜茫聙?

Agent Runtime SDK 氓聹篓忙聹卢忙聳聡盲赂颅盲赂聧忙聵炉忙聼聬盲赂陋 crate 莽職聞莽庐聙氓聧聲茅聡聧氓聭陆氓聬聧茂录聦猫聙聦忙聵炉盲赂聙莽禄聞氓聫炉氓炉鹿氓陇聳莽篓鲁氓庐職忙聣驴猫炉潞莽職聞猫驴聬猫隆聦忙聴露猫聝陆氓聤聸猫戮鹿莽聲聦茫聙?
莽聸庐忙聽聡莽聤露忙聙聛盲赂聥茂录聦猫掳聝莽聰篓忙聳鹿氓潞聰猫聝陆茅聙職猫驴聡莽篓鲁氓庐職 API 氓聢聸氓禄潞 runtime茫聙聛忙聫聬盲潞?turn茫聙聛忙露聢猫麓鹿盲潞聥盲禄露忙碌聛茫聙聛忙鲁篓氓聠?tool / harness / service
provider茫聙聛氓陇聞莽聬?permission / cancellation / persistence / telemetry茂录聦猫聙聦盲赂聧茅聹聙猫娄聛盲戮聺猫碌?`northhing-core`茫聙聛app crate茫聙?
Tauri handle 忙聢聳盲禄禄盲陆聲盲潞搂氓聯聛氓陆垄忙聙聛莽職聞 concrete manager茫聙聜氓聹篓猫炉楼莽聸庐忙聽聡猫戮戮忙聢聬氓聣聧茂录聦`execution` 氓卤聜氓聫陋猫聝陆莽搂掳盲赂潞忙聣搂猫隆聦氓聨聼猫炉颅茅聸聠氓聬聢茂录聦
盲赂聧猫聝陆氓炉鹿氓陇聳氓庐拢莽搂掳盲赂潞氓庐聦忙聲?SDK茫聙?

莽聸庐忙聽聡莽聤露忙聙聛氓驴聟茅隆禄盲驴聺忙聦聛盲潞搂氓聯聛猫隆聦盲赂潞茫聙聛茅禄聵猫庐陇猫聝陆氓聤聸茅聸聠氓聬聢茫聙聛忙聺聝茅聶聬猫炉颅盲鹿聣茫聙聛氓路楼氓聟路忙聸聺氓聟聣茫聙聛盲潞聥盲禄露猫炉颅盲鹿聣氓聮聦 release 忙聻聞氓禄潞氓陆垄忙聙聛莽颅聣盲禄路茫聙?

## 2. 忙聻露忙聻聞氓聨聼氓聢聶

- 盲戮聺猫碌聳氓聫陋猫聝陆盲禄聨盲潞搂氓聯聛氓聟楼氓聫?/ 盲潞搂氓聯聛莽禄聞猫拢聟忙碌聛氓聬聭盲潞搂氓聯聛猫聝陆氓聤聸茫聙聛氓聟路盲陆聯茅聙聜茅聟聧茫聙聛忙聹聧氓聤隆氓聮聦忙聣搂猫隆聦氓聨聼猫炉颅茂录聦氓聠聧忙碌聛氓聬聭莽篓鲁氓庐職氓楼聭莽潞娄茂录聸盲赂聥氓卤聜盲赂聧氓戮聴忙聞聼莽聼楼盲赂聤氓卤聜盲潞搂氓聯聛氓陆垄忙聙聛茫聙?
- 忙聨楼氓聫拢氓聮聦氓庐聻莽聨掳氓驴聟茅隆禄氓聢聠氓录聙茂录職忙聨楼氓聫拢氓卤聻盲潞聨莽篓鲁氓庐職氓楼聭莽潞娄茫聙聛Runtime Services茫聙聛Tool primitives 忙聢?Harness contract茂录?
  氓聟路盲陆聯氓庐聻莽聨掳氓卤聻盲潞聨 Product Assembly 莽職聞忙鲁篓氓聠聦猫戮鹿莽聲聦茫聙聛Adapters 忙聢?Services茫聙?
- Product interface 氓聫炉盲禄楼忙聹聣氓路庐氓录聜茂录聦capability contract 氓驴聟茅隆禄忙聰露忙聲聸茫聙聜盲赂聧氓聬聦盲潞搂氓聯聛氓聟楼氓聫拢氓聫炉盲禄楼茅聙聣忙聥漏盲赂聧氓聬聦猫聝陆氓聤聸茅聸聠氓聬聢茂录?
  盲陆聠盲赂聧猫聝陆茅聙職猫驴聡盲赂聥忙虏聣 UI茫聙聛氓聭陆盲禄陇忙聢聳氓聧聫猫庐庐茅聙禄猫戮聭忙聺楼忙聧垄氓聫聳氓陇聧莽聰篓茫聙?
- `northhing-core` 盲驴聺莽聲聶氓聟录氓庐鹿 facade 氓聮?`product-full` 莽禄聞猫拢聟猫戮鹿莽聲聦茂录聸忙聳掳 owner crate 盲赂聧氓戮聴盲戮聺猫碌聳氓聸?
  `northhing-core`茫聙?
- 氓炉鹿氓陇聳 SDK API 氓驴聟茅隆禄忙聵炉莽篓鲁氓庐職茫聙聛莽陋聞氓聫拢氓戮聞茫聙聛氓聫炉莽聣聢忙聹卢氓聦聳莽職聞 fa脙搂ade茂录聦盲赂聧氓戮聴忙聤聤 `northhing-core`茫聙聛`product-full`茫聙聛氓聟篓茅聡?
  service bundle 忙聢聳盲潞搂氓聯聛氓聠聟茅聝?manager 忙職麓茅聹虏莽禄聶猫掳聝莽聰篓忙聳鹿茫聙?
- Hook 忙聵炉氓聫聴忙聨搂忙聣漏氓卤聲莽聜鹿茂录聦Event 忙聵炉盲潞聥氓庐聻茅聙職莽聼楼茫聙聜猫聝陆忙聰鹿氓聫聵猫隆聦盲赂潞莽職?hook 氓驴聟茅隆禄忙聹聣茅隆潞氓潞聫茫聙聛timeout茫聙聛茅聰聶猫炉炉莽颅聳莽聲楼氓聮聦莽颅聣盲禄路盲驴聺忙聤陇茫聙?
- feature group 忙聵炉忙聻聞氓禄潞猫戮鹿莽聲聦茂录聦CapabilitySet 忙聵炉盲潞搂氓聯聛猫驴聬猫隆聦忙聴露猫聝陆氓聤聸猫戮鹿莽聲聦茂录聸盲赂陇猫聙聟氓驴聟茅隆禄莽聰卤 Product Assembly
  忙聵戮氓录聫忙聵聽氓掳聞茫聙?

## 3. 氓聢聺氓搂聥莽聤露忙聙聛茅聙禄猫戮聭猫搂聠氓聸戮

氓聢聺氓搂聥莽聤露忙聙聛莽職聞忙聽赂氓驴聝盲潞聥氓庐聻忙聵炉茂录職氓陇職盲赂陋 crate 氓路虏莽禄聫忙聣驴忙聨楼盲潞聠莽篓鲁氓庐職莽卤禄氓聻聥茫聙聛盲潞聥盲禄露茫聙聛stream茫聙聛tool contract茫聙聛茅聝篓氓聢?service
helper 氓聮?product domain 莽潞炉茅聙禄猫戮聭茂录聦盲陆聠氓庐聦忙聲麓猫驴聬猫隆聦忙聴露盲禄聧盲禄?`northhing-core` 盲赂潞盲赂颅氓驴聝茫聙?

```mermaid
flowchart TB
  Surfaces["盲潞搂氓聯聛氓聟楼氓聫拢<br/>Desktop / CLI / Server / Relay / Remote / Web"]
  Core["northhing-core<br/>氓聟录氓庐鹿 facade + 氓庐聦忙聲麓盲潞搂氓聯聛 runtime 莽禄聞猫拢聟"]
  Acp["northhing-acp<br/>ACP protocol surface / client behavior"]
  Transport["transport / api-layer<br/>API 盲赂聨盲录聽猫戮?adapter"]
  CoreTypes["northhing-core-types<br/>莽篓鲁氓庐職 DTO 氓颅聬茅聸聠"]
  Events["northhing-events<br/>盲潞聥盲禄露盲潞聥氓庐聻盲赂?emitter 忙聤陆猫卤隆"]
  Ports["northhing-runtime-ports<br/>trait-only runtime 猫戮鹿莽聲聦"]
  Stream["northhing-agent-stream<br/>stream 猫聛職氓聬聢"]
  AgentTools["northhing-agent-tools<br/>tool contract 盲赂聨莽潞炉莽颅聳莽聲楼"]
  ToolRuntime["tool-execution<br/>tool-runtime package / 盲陆聨氓卤聜 helper"]
  ToolPacks["tool-provider-groups<br/>northhing-tool-packs package / provider plan"]
  ServicesCore["northhing-services-core<br/>氓聼潞莽隆聙 service helper / filesystem facade"]
  ServicesIntegrations["northhing-services-integrations<br/>MCP / Git / Remote helper owner"]
  ProductDomains["northhing-product-domains<br/>MiniApp / function-agent 莽潞?domain"]
  Terminal["terminal-core<br/>terminal domain"]
  Ai["northhing-ai-adapters<br/>忙篓隆氓聻聥 provider adapter"]
  External["氓陇聳茅聝篓莽鲁禄莽禄聼<br/>OS / Git / MCP / ACP / AI provider / remote host"]

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

氓聢聺氓搂聥莽聤露忙聙聛盲赂禄猫娄聛忙篓隆氓聺聴猫聦聝氓聸麓茂录職

| 忙篓隆氓聺聴 | 氓聢聺氓搂聥氓庐職盲陆聧 | 忙聻露忙聻聞氓陆卤氓聯聧 |
|---|---|---|
| `northhing-core` | 氓聟录氓庐鹿 facade茫聙聛agent runtime茫聙聛tool runtime 莽禄聞猫拢聟茫聙聛service 忙聨楼莽潞驴氓聮聦氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸茅聸聠氓聬?| 盲禄聧忙聵炉盲潞聥氓庐聻盲赂聤莽職聞 runtime owner茂录聦忙聥聠猫搂拢氓驴聟茅隆禄氓聟聢盲驴聺忙聤陇猫隆聦盲赂潞莽颅聣盲禄路 |
| `northhing-runtime-ports` | 茅聺垄氓聬聭 runtime/service 猫戮鹿莽聲聦莽職?DTO 氓聮?trait | 氓聫陋氓庐職盲鹿?contract茂录聦盲赂聧忙聥楼忙聹聣 runtime 氓庐聻莽聨掳 |
| `tool-contracts` / `northhing-agent-tools` | provider-neutral tool DTO茫聙聛manifest茫聙聛path/result policy茫聙聛catalog contract 氓聮?deterministic execution admission gate | 茅聙聜氓聬聢忙聣驴忙聨楼莽潞?tool contract 莽颅聳莽聲楼茂录聦盲陆聠盲赂聧氓潞聰忙聥楼忙聹聣氓聟路盲陆聯 IO tool |
| `tool-execution` / `tool-runtime` | 忙聴垄忙聹聣盲陆聨氓卤聜氓路楼氓聟路忙聣搂猫隆聦 helper crate | 莽聸庐忙聽聡忙聵炉氓聫陋忙聣驴忙聨楼盲陆聨氓卤聜 file/search/tool execution helper茂录聦盲赂聧忙聥楼忙聹聣盲潞搂氓聯聛 registry 忙聢?permission policy |
| `northhing-services-core` | 氓聼潞莽隆聙 service helper茫聙聛忙聹卢氓聹?filesystem facade茫聙聛茅聝篓氓聢聠茅聙職莽聰篓 service 茅聙禄猫戮聭 | 茅聙聜氓聬聢盲陆聹盲赂潞忙聹卢氓聹掳氓聼潞莽隆聙 service owner茂录聦盲陆聠盲赂聧猫聝陆氓聬赂忙聰露盲潞搂氓聯聛 runtime 猫炉颅盲鹿聣 |
| `northhing-services-integrations` | MCP茫聙聛Git茫聙聛remote-connect茫聙聛remote-SSH 莽颅?integration helper | 茅聙聜氓聬聢忙聥楼忙聹聣氓陇聳茅聝篓氓聧聫猫庐庐氓聮聦茅聡聧盲戮聺猫碌聳 service implementation茂录聦盲赂聧氓潞聰氓聫聧氓聬聭忙聞聼莽聼楼盲潞搂氓聯?interface |
| `northhing-product-domains` | MiniApp茫聙聛function-agent 莽颅聣莽潞炉莽聤露忙聙聛茫聙聛莽颅聳莽聲楼茫聙聛port 氓聮聦茅聝篓氓聢聠氓聠鲁莽颅聳茅聙禄猫戮聭 | 茅聙聜氓聬聢忙聣驴忙聨楼 pure domain茂录聦盲赂聧氓潞聰莽聸麓忙聨楼忙聣搂猫隆?filesystem/Git/AI concrete call |
| `northhing-acp` | ACP protocol interface 氓聮?client behavior | 氓潞聰盲驴聺忙聦聛盲潞搂氓聯聛氓聧聫猫庐庐氓聟楼氓聫拢茂录聦盲赂聧盲赂聥忙虏聣氓聢掳 Agent Runtime |
| `transport` / `api-layer` | surface 氓聢?runtime 莽職?API/transport adapter | 氓潞聰盲驴聺忙聦聛盲录聽猫戮聯氓卤聜茂录聦盲赂聧忙聥楼忙聹聣 runtime owner |

## 4. 氓聢聺氓搂聥莽聤露忙聙聛盲赂禄猫娄聛茅聴庐茅垄?

### 4.1 氓聢聠氓卤聜盲赂聧忙赂聟忙聶?

氓聬聦盲赂聙猫聝陆氓聤聸莽禄聫氓赂赂氓聬聦忙聴露氓聦聟氓聬芦 UI/command茫聙聛runtime orchestration茫聙聛tool execution茫聙聛service IO 氓聮?domain
decision茫聙聜氓聢聺氓搂聥莽聤露忙聙聛盲禄拢莽聽聛盲赂颅猫驴聶盲潞聸茅聝篓氓聢聠盲禄聧氓陇搂茅聡聫茅聙職猫驴聡 `northhing-core` 盲赂虏猫聛聰茂录聦氓炉录猫聡麓忙聥聠猫搂拢忙聴露茅職戮盲禄楼氓聢陇忙聳颅芒聙聹莽搂禄氓聤篓莽職聞忙聵炉忙聨楼氓聫拢茫聙?
氓庐聻莽聨掳茫聙聛莽禄聞猫拢聟茅聙禄猫戮聭猫驴聵忙聵炉盲潞搂氓聯聛猫隆聦盲赂潞芒聙聺茫聙?

### 4.2 忙聨楼氓聫拢盲赂聨氓庐聻莽聨掳猫戮鹿莽聲聦盲赂聧莽篓鲁氓庐職

氓路虏忙聹聣 `runtime-ports` 氓聮聦猫聥楼氓鹿?contract crate茂录聦盲陆聠猫庐赂氓陇職 call site 盲禄聧盲戮聺猫碌?concrete manager茫聙?
core-owned context 忙聢聳氓庐聦忙聲?product runtime snapshot茫聙聜忙聨楼氓聫拢忙虏隆忙聹聣莽篓鲁氓庐職氓聢掳猫露鲁盲禄楼猫庐?runtime 盲赂聨氓聟路盲陆?service
氓庐聻莽聨掳莽聥卢莽芦聥忙录聰猫驴聸茫聙?

### 4.3 盲潞搂氓聯聛氓陆垄忙聙聛猫垄芦氓庐聦忙聲麓 core 莽聣碌氓录聲

Desktop茫聙聛CLI茫聙聛Server茫聙聛Remote茫聙聛ACP 氓聮?Web 莽職聞氓聟楼氓聫拢氓路庐氓录聜猫戮聝氓陇搂茂录聦盲陆聠氓聢聺氓搂聥莽聤露忙聙聛盲赂聥氓陇搂氓陇職盲禄聧茅聙職猫驴聡氓庐聦忙聲麓 `northhing-core`
猫聨路氓戮聴猫聝陆氓聤聸茫聙聜猫驴聶盲录職猫庐漏猫陆禄茅聡聫盲潞陇盲禄聵氓陆垄忙聙聛莽禄搂忙聣驴盲赂聧氓驴聟猫娄聛莽職?tool茫聙聛service茫聙聛UI 忙聢聳氓鹿鲁氓聫掳盲戮聺猫碌聳茫聙?

### 4.4 Tool contract 盲赂?tool execution 忙路路氓聬聢

provider-neutral manifest茫聙聛path policy茫聙聛result policy茫聙聛`ToolUseContext` runtime handle茫聙聛collapsed unlock
lifecycle茫聙聛runtime artifact persistence 氓聮?product registry materialization 氓聹篓氓聢聺氓搂聥莽聤露忙聙聛盲赂聥盲赂?concrete tool
execution 盲潞陇莽禄聡氓聹?core 氓聫聤氓聟露氓聟录氓庐鹿猫路炉氓戮聞盲赂颅茫聙聜莽聸庐忙聽聡莽聤露忙聙聛盲赂聥茂录聦tool contracts 氓潞聰忙聥楼忙聹?provider-neutral manifest /
catalog / permission / result / artifact contract茂录聦core茫聙聛services 忙聢?adapter 氓聫陋盲驴聺莽聲聶氓庐聻茅聶?IO tool adapter茫聙?
state update茫聙聛忙聴搂猫路炉氓戮聞 facade 氓聮聦忙聹聣莽颅聣盲禄路盲驴聺忙聤陇莽職聞忙聥聠猫搂拢猫戮鹿莽聲聦茫聙聜氓路楼氓聟?owner 忙聥聠猫搂拢氓娄聜忙聻聹忙虏隆忙聹聣氓驴芦莽聟搂盲驴聺忙聤陇茂录聦氓庐鹿忙聵聯忙聰鹿氓聫?
prompt-visible manifest茫聙聛`GetToolSpec`茫聙聛MCP/ACP catalog 忙聢?oversized result 猫隆聦盲赂潞茫聙?

### 4.5 Service茫聙聛MCP茫聙聛ACP 盲赂?runtime kernel 氓庐鹿忙聵聯盲潞陇氓聫聣

MCP 氓聮?ACP 忙聵炉氓陇聳茅聝篓氓聧聫猫庐?猫聝陆氓聤聸忙聨楼氓聟楼茂录聦盲赂聧氓潞聰氓聫聵忙聢?Agent Runtime SDK 莽職聞氓聠聟茅聝篓氓聧聫猫庐庐盲戮聺猫碌聳茫聙聜Runtime kernel 氓聫陋氓潞聰莽聹聥猫搂聛
external capability茫聙聛tool provider 忙聢?service port茂录聸猫驴聻忙聨楼莽聰聼氓聭陆氓聭篓忙聹聼茫聙聛茅聣麓忙聺聝茫聙聛transport 氓聮?timeout 莽颅聳莽聲楼氓潞聰莽聰卤
Adapters茫聙聛Services 忙聢?Product Assembly 莽庐隆莽聬聠茫聙?

### 4.6 忙聣漏氓卤聲莽聜鹿莽录潞氓掳聭莽禄聼盲赂聙猫炉颅盲鹿聣

agent definitions茫聙聛subagents茫聙聛skills茫聙聛prompt modules茫聙聛tool providers茫聙聛MCP providers茫聙聛hooks 氓聮?
product commands 茅聝陆忙聵炉忙聣漏氓卤聲莽聜鹿茂录聦盲陆聠莽聸庐氓聣聧忙虏隆忙聹聣莽禄聼盲赂聙猫隆篓猫戮戮氓庐聝盲禄卢氓聢聠氓聢芦氓卤聻盲潞聨氓聯陋盲赂聙氓卤聜茫聙聛氓娄聜盲陆聲忙鲁篓氓聠聦茫聙聛忙聵炉氓聬娄氓聟聛猫庐赂忙聰鹿氓聫聵猫隆聦盲赂潞茫聙?
盲禄楼氓聫聤氓娄聜盲陆聲氓聛職忙聺聝茅聶聬氓聮聦忙碌聥猫炉聲盲驴聺忙聤陇茫聙?

### 4.7 feature graph 猫驴聵盲赂聧忙聵炉盲潞搂氓聯聛猫聝陆氓聤聸莽聼漏茅聵?

氓聢聺氓搂聥莽聤露忙聙聛盲赂聥茂录聦`product-full` 忙聵炉氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸莽職聞氓庐聣氓聟篓莽陆聭茂录聦盲赂聧忙聵炉忙聹聙莽禄聢忙聦聣盲潞搂氓聯聛忙聥聠氓聢聠莽職?feature matrix茫聙聜莽聸麓忙聨楼氓聡聫猫陆禄茅禄聵猫庐?feature
忙聢聳忙聤聤 feature group 氓陆聯忙聢聬盲潞搂氓聯聛猫聝陆氓聤聸猫戮鹿莽聲聦茂录聦茅聝陆盲录職氓录聲氓聟楼忙聻聞氓禄潞氓陆垄忙聙聛氓聮聦氓聫聭氓赂聝猫聝陆氓聤聸忙录聜莽搂禄茫聙?

### 4.8 忙聻聞氓禄潞盲赂聨忙碌聥猫炉聲莽聣碌氓录聲猫驴聡氓陇?

茅聡聧盲戮聺猫碌聳氓聮聦氓庐聦忙聲麓 runtime 猫聛職氓聬聢氓聹?`northhing-core` 氓聭篓氓聸麓茂录聦氓炉录猫聡麓氓卤聙茅聝篓忙碌聥猫炉聲茫聙聛owner crate 忙碌聥猫炉聲氓聮聦猫陆禄茅聡聫盲潞搂氓聯聛氓聟楼氓聫拢氓庐鹿忙聵聯猫垄芦
盲赂聧莽聸赂氓聟鲁盲戮聺猫碌聳忙聥聳氓聟楼莽录聳猫炉聭氓聮聦茅聯戮忙聨楼猫路炉氓戮聞茫聙聜莽聸庐忙聽聡莽聤露忙聙聛氓驴聟茅隆禄猫庐漏盲戮聺猫碌聳忙聰露莽聸聤氓聫炉氓潞娄茅聡聫茂录聦氓聬聦忙聴露盲赂聧猫聝陆盲禄楼莽聣潞莽聣虏氓聤聼猫聝陆莽颅聣盲禄路忙聧垄氓聫聳忙聻聞氓禄潞忙聰露莽聸聤茫聙?

### 4.9 SDK 氓聫聭氓赂聝猫戮鹿莽聲聦盲赂聧猫露鲁

氓路虏忙聹聣 `northhing-agent-runtime`茫聙聛`northhing-runtime-services`茫聙聛`tool-contracts`茫聙聛`tool-execution`茫聙聛`northhing-harness`
氓聮?`runtime-ports` 莽颅?SDK 氓聙聶茅聙聣氓聨聼猫炉颅茂录聦盲陆聠莽录潞氓掳聭氓聫炉氓炉鹿氓陇聳忙聣驴猫炉潞莽職聞莽禄聼盲赂聙 runtime fa脙搂ade茫聙聛莽篓鲁氓庐職茅聰聶猫炉炉忙篓隆氓聻聥茫聙聛盲潞聥盲禄露忙碌聛氓聧聫猫庐庐茫聙?
provider 忙鲁篓氓聠聦猫戮鹿莽聲聦茫聙聛忙聦聛盲鹿聟氓聦聳/忙聛垄氓陇聧氓楼聭莽潞娄氓聮聦忙聹聙氓掳聫盲戮聺猫碌聳忙聻聞氓禄潞氓陆垄忙聙聛茫聙聜氓娄聜忙聻聹氓陇聳茅聝篓猫掳聝莽聰篓忙聳鹿盲禄聧茅聹聙猫娄聛莽聸麓忙聨楼莽聬聠猫搂?`northhing-core`茫聙?
`product-full`茫聙聛concrete service manager 忙聢聳盲潞搂氓聯聛氓聭陆盲禄陇猫路炉氓戮聞茂录聦猫炉麓忙聵聨 SDK 猫戮鹿莽聲聦氓掳職忙聹陋氓庐聦忙聢聬茫聙?

## 5. 氓炉鹿莽聟搂氓聢聠忙聻聬

忙聹卢猫聤聜氓聫陋忙聫聬莽聜录氓炉鹿 northhing 氓聢聠氓卤聜忙聹聣莽聰篓莽職聞忙聻露忙聻聞盲驴隆氓聫路茂录聦盲赂聧忙聤聤氓聟露盲禄聳茅隆鹿莽聸庐莽職聞氓庐聻莽聨掳氓陆垄忙聙聛莽聸麓忙聨楼氓陇聧氓聢露氓聢掳 northhing茫聙?

### 5.1 Claude Code 莽聸赂氓聟鲁氓庐聻莽聨掳氓聫聜猫聙?

Claude Code 莽聸赂氓聟鲁 Rust 氓庐聻莽聨掳氓聫聜猫聙聝盲赂颅茂录聦workspace 氓掳?CLI binary茫聙聛provider API茫聙聛runtime茫聙聛tools茫聙?
commands茫聙聛plugins茫聙聛telemetry 氓聮?mock harness 忙聥聠忙聢聬盲赂聧氓聬聦 crate茫聙聜氓聟露 `runtime` 猫麓聼猫麓拢 session茫聙聛config茫聙?
permission茫聙聛MCP茫聙聛prompt 氓聮?runtime loop茂录聸`tools` 猫麓聼猫麓拢 tool specs 盲赂聨忙聣搂猫隆聦茂录聸`commands` 猫麓聼猫麓拢 slash command
registry茂录聸`plugins` 猫麓聼猫麓拢 plugin metadata茫聙聛hook 氓聮?install/enable/disable surfaces茫聙聜猫炉楼莽禄聯忙聻聞猫炉麓忙聵聨茂录?

- 氓路楼氓聟路猫搂聞忙聽录茫聙聛氓聭陆盲禄?surface茫聙聛plugin/hook 氓聮?runtime loop 氓聫炉盲禄楼氓聢聠氓录聙忙录聰猫驴聸茫聙?
- permission茫聙聛MCP lifecycle茫聙聛task registry茫聙聛LSP registry 莽颅聣氓聫炉盲陆聹盲赂潞 runtime/service owner 莽庐隆莽聬聠茂录聦猫聙聦盲赂聧忙聵炉忙聲拢猫聬陆氓聹篓 UI茫聙?
- 氓娄聜忙聻聹 runtime crate 氓聬聦忙聴露氓聬赂忙聰露 session茫聙聛MCP茫聙聛permission茫聙聛prompt 氓聮?tool bridge茂录聦盲鹿聼盲录職氓聫聵忙聢聬忙聳掳莽職聞茅聡聧猫聛職氓聬聢莽聜鹿茫聙?

忙聙禄莽禄聯茂录職忙聥聠氓聢?crate 盲赂聧忙聵炉莽聸庐忙聽聡忙聹卢猫潞芦茂录聦氓聟鲁茅聰庐忙聵炉猫庐?CLI/TUI茫聙聛commands茫聙聛tools茫聙聛plugins茫聙聛runtime 氓聮?
service integrations 茅聙職猫驴聡莽篓鲁氓庐職 contract 莽禄聞氓聬聢茂录聦茅聛驴氓聟聧忙聤聤 `northhing-core` 莽職聞猫聛職氓聬聢茅聴庐茅垄聵忙聬卢氓聢掳忙聳掳莽職?runtime crate茫聙?

### 5.2 Opencode

Opencode 氓庐聵忙聳鹿忙聳聡忙隆拢氓卤聲莽陇潞盲潞聠忙聸麓氓聛聫盲潞搂氓聯聛氓聦聳莽職聞忙聣漏氓卤聲忙篓隆氓聻聥茂录職氓聬聦盲赂聙盲赂?agent 氓聫炉盲禄楼猫驴聬猫隆聦氓聹?terminal茫聙聛desktop 忙聢?IDE茂录?
agents 氓聢聠盲赂潞 primary agents 氓聮?subagents茂录聦氓聫炉茅聟聧莽陆庐 prompt茫聙聛model 盲赂?tool access茂录聸tools 茅聙職猫驴聡 permission 忙聨搂氓聢露茂录?
氓鹿露氓聫炉茅聙職猫驴聡 custom tools 忙聢?MCP servers 忙聣漏氓卤聲茂录聸plugins 猫庐垄茅聵聟 command茫聙聛file茫聙聛permission茫聙聛session茫聙聛tool茫聙聛TUI
莽颅聣盲潞聥盲禄露茂录聸skills 茅聙職猫驴聡莽聥卢莽芦聥莽聸庐氓陆聲忙聦聣茅聹聙氓聫聭莽聨掳氓聮聦氓聤聽猫陆陆茫聙?

忙聙禄莽禄聯茂录?

- Agent茫聙聛Tool茫聙聛MCP茫聙聛Plugin/Hook茫聙聛Skill 氓聮?Product Surface 氓潞聰猫炉楼忙聵炉盲潞聮莽聸赂猫驴聻忙聨楼莽職聞忙聣漏氓卤聲茅聺垄茂录聦猫聙聦盲赂聧忙聵炉氓聬聦盲赂聙盲赂陋忙篓隆氓聺聴氓聠聟茅聝篓莽職聞氓聢聠忙聰炉茫聙?
- 忙聺聝茅聶聬氓聮聦氓路楼氓聟路氓聫炉猫搂聛忙聙搂氓驴聟茅隆禄忙聵炉 runtime 氓聫炉猫搂聜忙碌聥莽職聞 contract茂录聦盲赂聧猫聝陆氓聫陋氓颅聵氓聹篓盲潞?UI 忙聢?prompt 忙聥录忙聨楼盲赂颅茫聙?
- 氓陇職盲潞搂氓聯聛氓陆垄忙聙聛茅聹聙猫娄?Product Assembly 氓聛?capability/provider 茅聙聣忙聥漏茂录聦猫聙聦盲赂聧忙聵炉猫庐漏 Agent Runtime SDK 氓聢陇忙聳颅猫掳聝莽聰篓忙聺楼猫聡陋
  Desktop茫聙聛CLI茫聙聛Remote 猫驴聵忙聵炉 ACP茫聙?

## 6. 莽聸庐忙聽聡茅聙禄猫戮聭猫搂聠氓聸戮

莽聸庐忙聽聡忙聻露忙聻聞盲禄楼氓聟颅盲赂陋莽聣漏莽聬?owner 氓聢聠氓聦潞猫隆篓猫戮戮盲戮聺猫碌聳忙聳鹿氓聬聭茫聙聜`interfaces` 氓聫陋忙聣驴猫陆陆氓聧聫猫庐庐氓聮聦氓庐驴盲赂禄氓聟楼氓聫拢茂录聸`assembly` 猫麓聼猫麓拢盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏盲赂聨忙鲁篓氓聠聦茂录聸`adapters` 猫麓聼猫麓拢氓聧聫猫庐庐茫聙聛transport 氓聮聦氓陇聳茅聝?provider 猫陆卢忙聧垄茂录聸`services` 猫麓聼猫麓拢忙聹卢氓聹掳莽鲁禄莽禄聼盲赂?runtime infrastructure 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳茂录聸`execution` 氓聫陋忙聰戮氓聫炉莽搂禄忙陇聧忙聣搂猫隆聦氓聨聼猫炉颅茂录聸`contracts` 忙聫聬盲戮聸莽篓鲁氓庐職盲潞聥氓庐聻茫聙聛port 氓聮聦盲潞搂氓聯聛茅垄聠氓聼聼猫搂聞氓聢聶茫聙聜猫驴聶忙聽路氓聫炉盲禄楼氓聬聦忙聴露氓聦潞氓聢聠芒聙聹氓聧聫猫庐庐茅聙聜茅聟聧芒聙聺氓聮聦芒聙聹忙聹聧氓聤隆氓庐聻莽聨掳芒聙聺茂录聦盲鹿聼茅聛驴氓聟聧忙聤聤 execution 猫炉炉猫搂拢盲赂潞氓庐聦忙聲麓猫驴聬猫隆聦忙聴露氓庐聻莽聨掳氓卤聜茫聙?

```mermaid
flowchart TB
  Interfaces["忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?br/>UI / command / protocol interface / delivery profile"]
  Assembly["盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?br/>compatibility facade / capability selection / adapter and service registration"]
  Adapters["茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?br/>AI / API / transport / WebDriver / external provider translation"]
  Services["忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?br/>filesystem / git / terminal / MCP / remote / process / OS integration"]
  Execution["忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?br/>agent / harness / stream / typed-service / tool primitives"]
  Contracts["莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?br/>DTO / event / runtime port / product domain policy"]
  External["氓陇聳茅聝篓莽鲁禄莽禄聼茂录聢External Systems茂录?br/>OS / Git / MCP server / ACP client / AI provider / remote host"]

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

盲戮聺猫碌聳忙聳鹿氓聬聭氓聫陋氓聟聛猫庐赂盲禄聨盲赂聤氓聢掳盲赂聥茫聙聜忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜忙職麓茅聹虏盲潞搂氓聯聛氓陆垄忙聙聛茂录聸莽禄聞猫拢聟氓卤聜茅聙聣忙聥漏猫聝陆氓聤聸茅聸聠氓聬聢氓鹿露忙鲁篓氓聠?adapter/service茂录聸茅聙聜茅聟聧氓卤聜莽驴禄猫炉聭氓聧聫猫庐庐氓聮聦氓陇聳茅聝篓 provider茂录聸忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜忙聨楼猫搂娄 OS茫聙聛process茫聙聛filesystem茫聙聛git茫聙聛terminal茫聙聛MCP 氓聮?remote茂录聸忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜忙聫聬盲戮聸氓聫炉氓陇聧莽聰?runtime building blocks茂录聸氓楼聭莽潞娄氓卤聜忙聫聬盲戮聸莽篓鲁氓庐職盲潞聥氓庐聻茫聙聛port 氓聮聦盲潞搂氓聯聛茅垄聠氓聼聼猫搂聞氓聢聶茫聙聜盲禄禄盲陆聲盲赂聥氓卤?crate 氓聫聧氓聬聭猫炉禄氓聫聳盲潞搂氓聯聛氓聟楼氓聫拢茫聙聛莽禄聞猫拢聟茅聟聧莽陆庐忙聢聳 host state 茅聝陆猫搂聠盲赂潞猫戮鹿莽聲聦猫驴聺猫搂聞茫聙?

## 7. 莽聸庐忙聽聡氓卤聜莽潞搂

莽聸庐忙聽聡氓卤聜莽潞搂盲禄楼莽聣漏莽聬?owner 氓聢聠氓聦潞盲赂潞氓聟楼氓聫拢茫聙聜忙炉聫盲赂陋氓聢聠氓聦潞氓聫炉盲禄楼氓聦聟氓聬芦氓陇職盲赂?crate茂录聦盲陆聠 crate 氓聠聟茅聝篓猫聛聦猫麓拢氓驴聟茅隆禄猫聝陆氓陇聼茅聙職猫驴聡盲戮聺猫碌聳茫聙聛忙碌聥猫炉聲氓聮聦猫戮鹿莽聲聦猫聞職忙聹卢莽聥卢莽芦聥茅陋聦猫炉聛茫聙?

### 7.1 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?

忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜忙聵炉莽聰篓忙聢路茫聙聛氓聧聫猫庐庐忙聢聳氓陇聳茅聝篓莽鲁禄莽禄聼猫驴聸氓聟楼 northhing 莽職聞氓聟楼氓聫拢茂录聦猫麓聼猫麓拢 UI茫聙聛氓聭陆盲禄陇茫聙聛猫路炉莽聰卤茫聙聛氓聧聫猫庐庐忙聨楼氓聫拢茫聙聛盲潞陇盲禄聵氓陆垄忙聙聛茅聙聣忙聥漏氓聮?host integration茫聙聜氓炉鹿氓潞聰猫聦聝氓聸麓氓聦聟忙聥?`src/apps/*`茫聙聛`src/web-ui`茫聙聛`src/mobile-web`茫聙聛`northhing-Installer`茫聙聛`tests/e2e` 氓聮?`src/crates/interfaces`茫聙聜氓聟楼氓聫拢氓卤聜氓聫炉盲禄楼茅聙聣忙聥漏 `DeliveryProfile` 氓鹿露猫掳聝莽聰?assembly 忙聢?adapter API茂录聦盲陆聠盲赂聧忙聥楼忙聹聣氓聟卤盲潞?runtime 猫隆聦盲赂潞茫聙?

### 7.2 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?

盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜猫麓聼猫麓拢氓聟录氓庐鹿氓炉录氓聡潞茫聙聛氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏茫聙聛feature group 氓聢?capability set 莽職聞忙聵聽氓掳聞茫聙聛adapter/service 忙鲁篓氓聠聦氓聮?product-full 忙聨楼莽潞驴茫聙聜莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/assembly`茂录聦氓陆聯氓聣聧氓聦聟氓聬?`northhing-core` 氓聟录氓庐鹿茅聴篓茅聺垄氓聮?`northhing-product-capabilities` 猫聝陆氓聤聸忙篓隆氓聻聥茫聙聜`product-capabilities` 氓聫陋忙聫聫猫驴?capability id茫聙聛tool group茫聙聛service requirement 氓聮?harness selection茂录聦盲赂聧忙聣搂猫隆聦 IO茂录聦盲鹿聼盲赂聧忙聣驴猫陆陆盲潞搂氓聯聛茅垄聠氓聼聼莽聤露忙聙聛忙聹潞茫聙?

### 7.3 茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?

茅聙聜茅聟聧氓卤聜猫麓聼猫麓拢氓聧聫猫庐庐茫聙聛transport茫聙聛氓陇聳茅聝?provider 氓聮聦氓庐驴盲赂禄茅聙職盲驴隆猫陆卢忙聧垄茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/adapters`茫聙聜氓聟露盲赂?`ai-adapters` 猫麓聼猫麓拢 AI provider 猫炉路忙卤聜/氓聯聧氓潞聰忙聵聽氓掳聞氓聮?provider stream 氓聧聫猫庐庐猫搂拢忙聻聬茂录聦猫搂拢忙聻聬莽禄聯忙聻聹氓潞聰猫陆卢忙聧垄盲赂?execution 氓卤聜忙聥楼忙聹聣莽職聞莽禄聼盲赂聙 stream 氓楼聭莽潞娄茂录聸`api-layer` 猫麓聼猫麓拢盲潞搂氓聯聛氓庐驴盲赂禄氓聟卤莽聰篓莽職聞氓聬聨莽芦?API adapter茂录聦`transport` 猫麓聼猫麓拢盲潞聥盲禄露忙聤聲茅聙聮氓聮聦 host transport adapter茂录聦`webdriver` 猫麓聼猫麓拢 WebDriver 氓聧聫猫庐庐氓聮聦忙碌聫猫搂聢氓聶篓猫聡陋氓聤篓氓聦?adapter茫聙聜茅聙聜茅聟聧氓卤聜盲赂聧忙聥楼忙聹聣盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏茂录聦盲鹿聼盲赂聧忙聣驴猫陆陆氓聫炉氓陇聧莽聰篓 OS service 氓庐聻莽聨掳茫聙?

### 7.4 忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?

忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜猫麓聼猫麓拢忙聨楼猫搂娄忙聹卢氓聹掳莽鲁禄莽禄聼氓聮聦 runtime infrastructure 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/services`茫聙聜氓聟露盲赂?`services-core` 忙聣驴猫陆陆猫陆禄茅聡聫 service primitive茂录聦`services-integrations` 忙聣驴猫陆陆 MCP茫聙聛Git茫聙聛remote茫聙聛file watch 氓聮聦盲潞搂氓聯聛茅垄聠氓聼?port 莽職聞氓聟路盲陆聯氓庐聻莽聨掳茂录聦`terminal` 忙聣驴猫陆陆 PTY茫聙聛shell integration 氓聮?terminal session infrastructure茫聙聜忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜氓聫炉盲禄楼氓庐聻莽聨掳 `contracts`茫聙聛`execution` 忙聢?`product-domains` 氓庐職盲鹿聣莽職?port茂录聦盲陆聠盲赂聧茅聙聣忙聥漏盲潞搂氓聯聛 profile茂录聦盲鹿聼盲赂聧莽聸麓忙聨楼忙職麓茅聹?UI/氓聧聫猫庐庐氓聟楼氓聫拢茫聙?

### 7.5 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?

忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜忙聫聬盲戮?provider-neutral 莽職?runtime building blocks茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/execution`茫聙聜`agent-runtime`茫聙聛`agent-stream`茫聙聛`harness`茫聙聛`runtime-services`茫聙聛`tool-contracts`茫聙聛`tool-provider-groups` 氓聮?`tool-execution` 氓聢聠氓聢芦氓庐職盲鹿聣 agent loop facts茫聙聛莽禄聼盲赂聙 stream DTO / tool-call 莽麓炉莽搂炉 / replay 氓楼聭莽潞娄茫聙聛workflow descriptor茫聙聛typed service bundle茫聙聛tool manifest / permission / result policy茫聙聛tool group facts 氓聮聦盲陆聨氓卤?tool execution helper茫聙聜氓陆聯氓聣?Cargo package / lib 氓聬聧盲驴聺忙聦聛氓聟录氓庐鹿茂录聦盲陆聠莽聣漏莽聬聠莽聸庐氓陆聲忙聦聣猫聛聦猫麓拢氓聭陆氓聬聧茫聙聜氓庐聝盲禄卢氓聫陋猫聝陆盲戮聺猫碌聳莽篓鲁氓庐職氓楼聭莽潞娄忙聢聳忙聵聨莽隆庐莽職?provider-neutral DTO茂录聦盲赂聧莽聸麓忙聨楼氓聢聸氓禄潞 Tauri handle茫聙聛filesystem manager茫聙聛Git provider茫聙聛MCP client茫聙聛AI client 忙聢?host process茫聙?

### 7.6 莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?

莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜忙聵炉忙聹聙盲陆聨氓卤聜茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/contracts`茫聙聜氓庐聝氓聦聟氓聬芦 `core-types`茫聙聛`events`茫聙聛`runtime-ports` 氓聮?`product-domains`茫聙聜`product-domains` 忙聵?Product Domain Model茂录聦猫麓聼猫麓?MiniApp茫聙聛function-agent 莽颅聣茅垄聠氓聼?DTO茫聙聛莽潞炉莽颅聳莽聲楼茫聙聛莽聤露忙聙聛猫搂聞氓聢聶氓聮聦莽陋?port茂录聸氓聟路盲陆?Git茫聙聛filesystem茫聙聛AI 忙聢?worker execution 氓庐聻莽聨掳氓聹?services茫聙聛adapters 忙聢?assembly/core 莽職聞氓聟录氓庐鹿猫路炉氓戮聞盲赂颅茂录聦盲赂聧氓戮聴氓聸聻忙碌聛氓聢掳 contracts茫聙?

### 7.7 忙聣漏氓卤聲莽聜鹿氓陆聮氓卤?

- AI茫聙聛API茫聙聛transport 氓聮?WebDriver 莽職聞氓聧聫猫庐庐猫陆卢忙聧垄氓卤聻盲潞?Adapters茫聙?
- MCP茫聙聛terminal茫聙聛filesystem茫聙聛git茫聙聛remote 氓聮?file watch 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳氓卤聻盲潞聨 Services茫聙?
- Tool manifest茫聙聛permission茫聙聛execution admission茫聙聛result / artifact policy 氓卤聻盲潞聨 Execution Primitives 莽職?`tool-contracts`茫聙?
- Tool provider group facts 氓卤聻盲潞聨 Execution Primitives 莽職?`tool-provider-groups`茂录聸盲陆聨氓卤?filesystem/search helper 氓卤聻盲潞聨 `tool-execution`茫聙?
- Agent茫聙聛subagent茫聙聛prompt module茫聙聛scheduler茫聙聛session / turn facts 氓聮?hook routing 氓卤聻盲潞聨 Execution Primitives茫聙?
- Harness workflow descriptor 氓聮?route plan 氓卤聻盲潞聨 Execution Primitives茂录聸氓聟路盲陆聯氓路楼盲陆聹忙碌聛 IO 莽聲聶氓聹篓 Services茫聙聛Adapters 忙聢聳氓聟录氓庐鹿猫路炉氓戮聞茂录聦莽聸麓氓聢掳忙聹聣莽颅聣盲禄路盲驴聺忙聤陇氓聬聨氓聠聧猫驴聛莽搂禄茫聙?
- Capability pack茫聙聛delivery profile茫聙聛adapter/service selection 氓聮?product-full assembly 氓卤聻盲潞聨 Product Assembly茫聙?
- 盲潞搂氓聯聛茅垄聠氓聼聼莽聤露忙聙聛茫聙聛猫搂聞氓聢聶茫聙聛port 氓聮?domain policy 氓卤聻盲潞聨 Stable Contracts and Product Domains茫聙?

## 8. 忙聨楼氓聫拢盲赂聨氓庐聻莽聨掳氓聟鲁莽鲁?

忙聨楼氓聫拢莽聰卤莽篓鲁氓庐職氓楼聭莽潞娄茫聙聛Runtime Services茫聙聛Tool Contracts 忙聢?Harness contract 氓庐職盲鹿聣茂录聸氓聟路盲陆聯氓庐聻莽聨掳莽聰卤 adapter茫聙聛service 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢氓聢聸氓禄潞茂录聸忙鲁篓氓聠聦氓聤篓盲陆聹氓聫陋猫聝陆氓聫聭莽聰聼氓聹?Product Assembly茫聙聜Agent Runtime茫聙聛tool contracts茫聙聛tool execution 氓聮?Harness 氓聫陋忙聨楼忙聰露氓路虏莽禄聫莽禄聞猫拢聟氓楼陆莽職聞忙聨楼氓聫拢忙聢聳 provider registry茂录聦盲赂聧莽聸麓忙聨楼氓聢聸氓禄潞氓鹿鲁氓聫掳氓庐聻莽聨掳茫聙?

```mermaid
flowchart TB
  Interface["忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?br/>茅聙聣忙聥漏氓聟楼氓聫拢氓聮?DeliveryProfile"]
  Assembly["盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?br/>氓聰炉盲赂聙忙鲁篓氓聠聦莽聜?]
  ServiceBuilder["猫驴聬猫隆聦忙聴露忙聹聧氓聤隆氓卤聜茂录聢Runtime Services茂录?br/>RuntimeServicesBuilder"]
  ToolBuilder["氓路楼氓聟路忙聣搂猫隆聦氓聨聼猫炉颅茂录聢Tool Primitives茂录?br/>tool contracts / groups / execution"]
  HarnessBuilder["氓路楼盲陆聹忙碌聛莽录聳忙聨聮氓卤聜茂录聢Harness Layer茂录?br/>HarnessRegistryBuilder"]
  AgentRegistry["Agent 忙聣搂猫隆聦氓聨聼猫炉颅茂录聢Agent Runtime茂录?br/>AgentDefinitionRegistry"]
  CommandRegistry["忙聨楼氓聫拢 / 盲潞搂氓聯聛莽禄聞猫拢聟氓卤?br/>ProductCommandRegistry"]
  Runtime["Agent / Tool / Harness primitives<br/>氓聫陋忙露聢猫麓鹿忙聨楼氓聫?]
  Adapters["茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?br/>AI / API / transport / WebDriver adapters"]
  Services["忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?br/>OS / filesystem / Git / terminal / MCP / remote services"]
  Contracts["莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?br/>DTO / event / port trait"]

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

忙鲁篓氓聠聦氓聶篓盲赂聨氓聣聧忙聳聡莽聸庐忙聽聡氓卤聜莽潞搂莽職聞氓炉鹿氓潞聰氓聟鲁莽鲁禄氓娄聜盲赂聥茂录職

| 忙鲁篓氓聠聦氓聶?/ 莽禄聞猫拢聟莽聜?| 忙聣聙氓卤聻莽聸庐忙聽聡氓卤聜莽潞?| 氓聢聺氓搂聥忙聣驴猫陆陆盲赂聨莽聸庐忙聽聡忙聣驴猫陆?| 忙鲁篓氓聠聦氓聠聟氓庐鹿 |
|---|---|---|---|
| `ProductAssembler` / `ProductAssemblyPlan` | 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?| 氓聢聺氓搂聥氓聫炉氓聹篓 `northhing-core` facade 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢茂录聸莽聸庐忙聽聡氓聫炉忙聰露忙聲聸盲赂潞 assembly owner | `DeliveryProfile`茫聙聛`CapabilitySet`茫聙聛feature group茫聙聛adapter/service 茅聙聣忙聥漏 |
| `RuntimeServicesBuilder` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录聣盲赂聨忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录聣莽職聞猫戮鹿莽聲聦 | 莽聸庐忙聽聡氓聹?`northhing-runtime-services`茂录聸猫驴聻忙聨?`northhing-runtime-ports`茫聙聛`northhing-services-*` 氓聮聦氓聢聺氓搂?service wiring | filesystem茫聙聛workspace茫聙聛session store茫聙聛Git茫聙聛terminal茫聙聛network茫聙聛MCP catalog茫聙聛remote connection / workspace / projection port |
| `ToolRuntimeBuilder` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?| `tool-execution`茫聙聛`tool-contracts`茫聙聛`tool-provider-groups`茂录聸Cargo package 氓聬聧盲驴聺忙聦聛氓聟录氓庐?| tool provider茫聙聛tool group茫聙聛manifest茫聙聛permission gate茫聙聛tool hook |
| `HarnessRegistryBuilder` | 氓路楼盲陆聹忙碌聛莽录聳忙聨聮氓卤聜茂录聢Harness Layer茂录?| 莽聸庐忙聽聡氓聹?`northhing-harness`茂录聸氓聢聺氓搂聥氓聫炉莽聰?`northhing-core::agentic::harness` 忙鲁篓氓聠聦 legacy-facade provider | SDD茫聙聛Deep Review茫聙聛DeepResearch茫聙聛MiniApp 莽颅?harness provider |
| `AgentDefinitionRegistry` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?| 莽聸庐忙聽聡氓聹?`northhing-agent-runtime`茂录聸氓聢聺氓搂聥氓聫炉莽聰?`northhing-core` agent definition 盲禄拢莽聽聛忙聣驴猫陆陆 | agent茫聙聛subagent茫聙聛prompt module茫聙聛skill definition |
| `ProductCommandRegistry` | 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录聣盲赂聨盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣莽職聞猫戮鹿莽聲聦 | 盲潞搂氓聯聛氓聟楼氓聫拢忙聢?assembly 忙篓隆氓聺聴 | 猫戮聯氓聟楼忙隆聠氓聭陆盲禄陇茫聙聛氓庐隆忙聽赂氓聟楼氓聫拢茫聙聛MiniApp 氓聟楼氓聫拢氓聢?capability / harness / runtime request 莽職聞忙聵聽氓掳?|
| adapter set | 茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?| `northhing-ai-adapters`茫聙聛`northhing-api-layer`茫聙聛`northhing-transport`茫聙聛`northhing-webdriver`茫聙聛app adapters | AI茫聙聛API茫聙聛transport茫聙聛WebDriver 莽颅聣氓聧聫猫庐庐忙聢聳氓陇聳茅聝篓 provider adapter |
| service set | 忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?| `northhing-services-*`茫聙聛`terminal-core` 氓聮聦氓聟路盲陆?app service implementations | OS茫聙聛filesystem茫聙聛Git茫聙聛terminal茫聙聛MCP茫聙聛remote 莽職聞氓聟路盲陆?service茂录聸Remote service 氓聠聟茅聝篓莽禄搂莽禄颅氓聦潞氓聢聠 SSH茫聙聛relay茫聙聛忙聹卢氓聹掳茅職搂茅聛聯茫聙聛猫驴聹莽芦?OS 忙聰炉忙聦聛 |

忙鲁篓氓聠聦猫路炉氓戮聞氓驴聟茅隆禄忙聵炉忙聵戮氓录聫茫聙聛typed茫聙聛氓聫炉忙碌聥猫炉聲莽職聞茂录職

- 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录聣氓聫陋茅聙聣忙聥漏 `DeliveryProfile` 氓聮聦盲潞搂氓聯聛茅聟聧莽陆庐茂录聦盲赂聧莽聸麓忙聨楼忙聤聤 concrete manager 盲录聽氓聟楼 runtime茫聙?
- 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣忙聽鹿忙聧庐盲潞搂氓聯聛氓陆垄忙聙聛氓聢聸氓禄潞忙聢聳忙聨楼忙聰露 adapter/service茂录聦氓鹿露猫掳聝莽聰篓 typed builder 氓庐聦忙聢聬忙鲁篓氓聠聦茫聙?
- Tool茫聙聛OS茫聙聛Remote茫聙聛Protocol provider 氓聢聠氓聢芦莽聲聶氓聹篓氓炉鹿氓潞聰 app茫聙聛Adapters 忙聢?Services 盲赂颅茂录聦茅聙職猫驴聡氓聬聦盲赂聙莽禄?port 忙職麓茅聹虏茫聙?
- Tauri 氓聫陋猫聝陆氓聡潞莽聨掳氓聹?Desktop app茫聙聛transport/API adapter 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢氓聭陆盲禄陇氓陇聳猫搂聜盲赂颅茂录聸Agent Runtime茫聙?
  Tool primitives茫聙聛Harness茫聙聛Runtime Services contract 氓聮?Product Capabilities 盲赂聧氓戮聴盲戮聺猫碌聳 Tauri handle茫聙?
  window茫聙聛command macro 忙聢?desktop app state茫聙?
- Remote provider 氓驴聟茅隆禄忙聥聠氓聢聠莽篓鲁氓庐職猫驴聻忙聨楼忙聨楼氓聫拢氓聮聦氓聟路盲陆聯猫驴聹莽芦?OS / transport 氓庐聻莽聨掳茂录聦茅聛驴氓聟聧忙聤聤 SSH茫聙聛relay 忙聢聳猫驴聹莽芦炉氓鹿鲁氓聫掳氓路庐氓录聜忙鲁聞忙录聫氓聢掳 runtime茫聙?
- 盲赂聧忙聰炉忙聦聛莽職聞猫聝陆氓聤聸氓聹?assembly 莽職?capability availability 盲赂颅忙聵戮氓录聫猫驴聰氓聸?unsupported / unavailable茂录聦盲赂聧氓聹?execution primitive 氓聠聟氓聠聶盲潞搂氓聯聛氓聢聠忙聰炉茫聙?
- 莽娄聛忙颅垄盲陆驴莽聰篓忙聴聽莽卤禄氓聻?`Any` service locator茫聙聛氓聟篓氓卤聙 mutable registry 忙聢聳盲赂聥氓卤?crate 氓聫聧氓聬聭猫炉禄氓聫聳盲潞搂氓聯聛茅聟聧莽陆庐茫聙?

## 9. 茅拢聨茅聶漏

| 茅拢聨茅聶漏 | 盲驴聺忙聤陇忙聳鹿氓录聫 |
|---|---|
| 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣猫聠篓猫聝聙盲赂潞忙聳掳莽職聞氓聟篓氓卤聙莽聤露忙聙聛盲赂颅氓驴?| assembly 氓聫陋氓聛職忙聻聞氓禄潞忙聹聼忙鲁篓氓聠聦茂录聦猫戮聯氓聡潞盲赂聧氓聫炉氓聫?runtime parts茂录聸盲潞搂氓聯聛莽聤露忙聙聛盲禄聧氓陆?surface 忙聢?runtime owner |
| 忙聨楼氓聫拢忙聥聠氓戮聴猫驴聡莽禄聠茂录聦氓炉录猫聡麓氓陇聧忙聺聜氓潞娄氓聮聦氓聤篓忙聙聛氓聢聠氓聫聭忙聢聬忙聹卢盲赂聤氓聧?| 盲禄?capability 氓聮聦莽篓鲁氓庐職莽聰篓盲戮聥氓庐職盲鹿?port 莽虏聮氓潞娄茂录聦莽聝颅猫路炉氓戮聞茅聛驴氓聟聧猫驴聬猫隆聦忙聴?map lookup茂录聦盲录聵氓聟?builder-time 忙鲁篓氓聟楼 |
| 氓鹿鲁氓聫掳氓庐聻莽聨掳忙鲁聞忙录聫氓聢?Agent茫聙聛Tool 忙聢?Harness execution primitives | 盲戮聺猫碌聳忙拢聙忙聼楼莽娄聛忙颅?execution owner 盲戮聺猫碌聳 app crate茫聙聛Tauri茫聙聛CLI TUI茫聙聛ACP protocol 氓聮?concrete service crate |
| core 忙聥聠氓聢聠氓聬聨盲禄聧茅職聬氓录聫莽禄聭氓庐職 Tauri | Tauri 氓聫陋氓聟聛猫庐赂氓聹篓 Desktop app 忙聢聳忙聵聨莽隆?feature-gated adapter茂录聸氓聬聭盲赂聥氓卤聜盲录聽茅聙?typed port茫聙聛DTO茫聙聛event fact 氓聮?capability availability |
| 盲赂聧氓聬聦盲潞搂氓聯聛氓陆垄忙聙聛猫聝陆氓聤聸莽聼漏茅聵碌忙录聜莽搂?| Product Assembly 莽禄麓忙聤陇 capability matrix茂录聸氓聡聫氓掳聭忙聢聳忙聸驴忙聧垄猫聝陆氓聤聸忙聴露猫隆楼盲潞搂氓聯聛氓聟楼氓聫拢茅陋聦猫炉聛氓聮?unsupported 猫隆聦盲赂潞忙碌聥猫炉聲 |
| Tool茫聙聛MCP茫聙聛ACP 莽職?manifest茫聙聛permission 忙聢聳盲潞聥盲禄露猫炉颅盲鹿聣忙聥聠猫搂拢氓聬聨盲赂聧莽颅聣盲禄?| 盲驴聺莽聲聶忙聴搂猫路炉氓戮聞氓聟录氓庐?facade茂录聦氓垄聻氓聤?manifest snapshot茫聙聛permission 氓聠鲁莽颅聳氓聮聦盲潞聥盲禄露忙聵聽氓掳聞莽颅聣盲禄路忙碌聥猫炉?|
| Harness provider 氓聫陋氓聛職忙鲁篓氓聠聦盲陆聠猫垄芦猫炉炉猫庐陇盲赂潞氓路虏莽禄聫忙聥楼忙聹聣忙聣搂猫隆聦猫炉颅盲鹿?| descriptor-only / legacy-facade provider 氓聫陋猫聝陆莽聰聼忙聢聬 route plan茂录聸忙聣搂猫隆聦猫炉颅盲鹿聣莽搂禄氓聤篓氓驴聟茅隆禄氓聧聲莽聥卢猫炉聛忙聵聨猫隆聦盲赂潞莽颅聣盲禄?|
| `northhing-core` 氓聫陋忙聵炉忙聰鹿氓聬聧盲赂潞忙聳掳莽職聞氓路篓氓聻?runtime crate | 忙聳?owner crate 氓驴聟茅隆禄忙聹聣氓聧聲盲赂聙猫聛聦猫麓拢氓聮聦忙聹聙氓掳聫盲戮聺猫碌聳茂录聸盲潞搂氓聯聛猫聝陆氓聤聸茫聙聛harness茫聙聛service 氓庐聻莽聨掳盲赂聧氓戮聴莽禄搂莽禄颅氓聽聠氓聟楼 agent kernel |
| 莽聸庐忙聽聡 crate 氓聟聢猫隆聦氓聢聸氓禄潞盲陆聠忙虏隆忙聹聣莽聹聼氓庐?owner | 氓聫陋忙聹聣 owner 猫戮鹿莽聲聦茫聙聛忙聴搂猫路炉氓戮聞氓聟录氓庐鹿茫聙聛focused tests茫聙聛盲戮聺猫碌聳忙聰露莽聸聤氓聮聦 boundary check 氓聬聦忙聴露忙聢聬莽芦聥忙聴露忙聣聧氓聢聸氓禄潞 crate茂录聸氓聬娄氓聢聶莽禄搂莽禄颅莽聲聶氓聹?facade |

## 10. 莽聸庐忙聽聡莽聤露忙聙聛氓聢陇氓庐?

- `northhing-core` 盲赂聧氓聠聧忙聵炉盲潞聥氓庐聻盲赂聤莽職聞氓庐聦忙聲?runtime owner茂录聦猫聙聦忙聵炉氓聟录氓庐鹿 facade 氓聮?`product-full` 莽禄聞猫拢聟猫戮鹿莽聲聦茫聙?
- Agent Runtime SDK 氓聫炉氓聹篓盲赂聧盲戮聺猫碌?`northhing-core`茫聙聛app crate 忙聢?Tauri 莽職聞忙聝聟氓聠碌盲赂聥猫垄芦氓碌聦氓聟楼茂录聦氓鹿露茅聙職猫驴聡莽篓鲁氓庐職 builder /
  runner / event stream / registry API 忙聫聬盲戮聸 agent 猫聝陆氓聤聸茫聙?
- Agent Runtime茫聙聛Tool Contracts / Tool Provider Groups / Tool Execution茫聙聛Runtime Services茫聙聛Harness 氓聮?Product Capabilities 氓聢聠氓聢芦忙聥楼忙聹聣氓聫炉氓庐隆忙聼楼莽職聞猫聛聦猫麓拢猫戮鹿莽聲聦茫聙?
- 莽篓鲁氓庐職氓楼聭莽潞娄氓聮聦氓聬聞 execution owner 氓庐職盲鹿聣忙聨楼氓聫拢茂录聸氓聟路盲陆?Tool茫聙聛OS茫聙聛Remote service 莽聲聶氓聹篓 Services茂录聦氓聧聫猫庐庐氓聮聦氓陇聳茅聝篓 provider 猫陆卢忙聧垄莽聲聶氓聹篓 Adapters茫聙?
- 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣忙聵炉氓聰炉盲赂聙忙鲁篓氓聠聦莽聜鹿茂录聦茅聙職猫驴聡 typed builder / registry 猫驴聻忙聨楼忙聨楼氓聫拢氓聮聦氓聟路盲陆聯氓庐聻莽聨掳茫聙?
- Tauri 氓聫陋氓卤聻盲潞?Desktop app 忙聢聳忙聵聨莽隆?feature-gated adapter茂录聦盲赂聧猫驴聸氓聟楼 core茫聙聛execution owner 忙聢?contract crate茫聙?
- runtime 氓聫陋盲戮聺猫碌?remote connection茫聙聛remote workspace茫聙聛remote projection 氓聮?capability facts 莽颅?port茂录聸SSH茫聙聛relay茫聙?
  忙聹卢氓聹掳茅職搂茅聛聯茫聙聛猫驴聹莽芦?OS 氓路庐氓录聜氓聮聦猫庐陇猫炉聛忙聳鹿氓录聫氓卤聻盲潞聨氓聟路盲陆?Remote provider茫聙?
- 盲潞搂氓聯聛氓陆垄忙聙聛氓路庐氓录聜茅聙職猫驴聡 capability matrix 氓聮?Product Assembly 猫隆篓猫戮戮茂录聦盲赂聧茅聙職猫驴聡盲赂聥忙虏聣 UI茫聙聛氓聭陆盲禄陇茫聙聛氓聧聫猫庐庐忙聢聳氓鹿鲁氓聫掳氓庐聻莽聨掳猫隆篓猫戮戮茫聙?
- 忙聺聝茅聶聬茫聙聛氓路楼氓聟路忙聸聺氓聟聣茫聙聛盲潞聥盲禄露茫聙聛session茫聙聛remote workspace 氓聮?release 忙聻聞氓禄潞氓陆垄忙聙聛氓驴聟茅隆禄盲驴聺忙聦聛氓聤聼猫聝陆莽颅聣盲禄路茫聙?
