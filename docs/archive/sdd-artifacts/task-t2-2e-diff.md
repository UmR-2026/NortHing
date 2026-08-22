BASE: 805cb0f (working-tree diff, task not yet committed)

## git diff --stat
 Cargo.lock                                         |  56 --
 scripts/core-boundaries/rules/crate-rules.mjs      |   1 -
 scripts/core-boundaries/rules/feature-rules.mjs    |  29 +-
 .../rules/source/required-rules.mjs                | 647 -------------------
 scripts/core-boundaries/self-test.mjs              |  91 ---
 src/crates/services/AGENTS-CN.md                   |   2 +-
 src/crates/services/AGENTS.md                      |   2 +-
 .../services/services-integrations/AGENTS.md       |   4 -
 .../services/services-integrations/Cargo.toml      |  35 --
 .../services/services-integrations/src/lib.rs      |   3 -
 .../src/remote_connect/device.rs                   |  71 ---
 .../src/remote_connect/encryption.rs               | 176 ------
 .../src/remote_connect/mod.rs                      | 437 -------------
 .../src/remote_connect/pairing.rs                  | 285 ---------
 .../src/remote_connect/qr_generator.rs             |  74 ---
 .../src/remote_connect/relay_client.rs             | 490 ---------------
 .../src/remote_connect/remote_cancel_handlers.rs   |  92 ---
 .../src/remote_connect/remote_dialog_handlers.rs   | 176 ------
 .../src/remote_connect/remote_file_io.rs           | 159 -----
 .../src/remote_connect/remote_request_builders.rs  | 616 ------------------
 .../src/remote_connect/remote_session_handlers.rs  | 624 ------------------
 .../remote_session_response_builders.rs            | 536 ----------------
 .../src/remote_connect/remote_session_state.rs     | 695 ---------------------
 .../remote_connect/remote_workspace_resolver.rs    |  99 ---
 .../services-integrations/tests/command_runtime.rs | 119 ----
 .../services-integrations/tests/common/mod.rs      | 526 +---------------
 .../tests/dialog_cancel_contracts.rs               | 290 ---------
 .../services-integrations/tests/file_transfer.rs   | 186 ------
 .../tests/model_catalog_tracker_poll.rs            | 449 -------------
 .../tests/pairing_qr_relay.rs                      |  67 --
 .../tests/session_wire_and_responses.rs            | 424 -------------
 .../tests/submission_images.rs                     | 189 ------
 32 files changed, 13 insertions(+), 7637 deletions(-)

## Deleted files (whole-file; all 7 test files were cfg(remote-connect) gated - header proof)
### tests/pairing_qr_relay.rs
//! Pairing Qr Relay contract tests.

#![cfg(feature = "remote-connect")]

mod common;

### tests/command_runtime.rs
//! Command Runtime contract tests.

#![cfg(feature = "remote-connect")]

mod common;

### tests/dialog_cancel_contracts.rs
//! Dialog Cancel Contracts contract tests.

#![cfg(feature = "remote-connect")]

mod common;

### tests/file_transfer.rs
//! File Transfer contract tests.

#![cfg(feature = "remote-connect")]

mod common;

### tests/model_catalog_tracker_poll.rs
//! Model Catalog Tracker Poll contract tests.

#![cfg(feature = "remote-connect")]

mod common;

### tests/session_wire_and_responses.rs
//! Session Wire And Responses contract tests.

#![cfg(feature = "remote-connect")]

mod common;

### tests/submission_images.rs
//! Submission Images contract tests.

#![cfg(feature = "remote-connect")]

mod common;

### src/remote_connect/ (14 files, 4081 lines) - module dir deleted; mod.rs head:
//! Remote-connect integration contracts (Round 11b split).
//!
//! This module owns remote-connect wire assembly, runtime-port request
//! construction, compatibility re-exports, and remote session tracker state.
//!
//! Round 11 split: 5 sibling files own domain-specific fns by prefix cluster.
//!
//! Round 11b split (QClaw R11 REQUIRED): the 2 over-cap files
//! `remote_command_handlers.rs` (1301) and `remote_session_tracker.rs` (1272)
//! are split into 5 new siblings, each 鈮?800 lines:
//! - `remote_session_state` (~700): `RemoteSessionStateTracker` + Registry +
//!   TrackerState + TrackerEvent + ActiveTurnSnapshot
//! - `remote_session_response_builders` (~600): `SessionInfo` + response
//!   builders + session/poll/initial-sync handlers + handler traits
//! - `remote_dialog_handlers` (~200): dialog submission types and fn
//! - `remote_cancel_handlers` (~110): cancel-task types and fn
//! - `remote_session_handlers` (~700): wire `RemoteCommand`/`RemoteResponse`
//!   re-exports + workspace/file/interaction sub-handlers + integration tests
//!
//! The wire enums + `RemoteCommandRuntimeHost` + top `handle_remote_command`

## git diff -U10 (modified files only)
diff --git a/Cargo.lock b/Cargo.lock
index 599ca02..ba0a075 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -3838,31 +3838,20 @@ dependencies = [
 
 [[package]]
 name = "home"
 version = "0.5.12"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "cc627f471c528ff0c4a49e1d5e60450c8f6461dd6d10ba9dcd3a61d3dff7728d"
 dependencies = [
  "windows-sys 0.61.2",
 ]
 
-[[package]]
-name = "hostname"
-version = "0.4.2"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "617aaa3557aef3810a6369d0a99fac8a080891b68bd9f9812a1eeda0c0730cbd"
-dependencies = [
- "cfg-if",
- "libc",
- "windows-link 0.2.1",
-]
-
 [[package]]
 name = "htmd"
 version = "0.5.4"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "7eee9b00ee2e599b4f86507157e3db786e7a3319fc225f0e9584151dbea2291d"
 dependencies = [
  "html5ever 0.38.0",
  "markup5ever_rcdom",
  "phf 0.13.1",
 ]
@@ -6201,55 +6190,45 @@ version = "0.2.10"
 dependencies = [
  "aes-gcm",
  "anyhow",
  "async-trait",
  "base64 0.22.1",
  "chrono",
  "dirs",
  "dunce",
  "futures",
  "git2",
- "hostname",
- "image",
- "mac_address",
  "northhing-events",
  "northhing-product-domains",
  "northhing-runtime-ports",
  "northhing-services-core",
  "northhing-test-support",
  "notify",
- "qrcode",
  "rand 0.8.7",
  "reqwest",
  "rmcp",
  "russh",
  "russh-keys",
  "russh-sftp",
- "rustls",
- "rustls-native-certs",
- "schannel",
  "serde",
  "serde_json",
  "sha2",
  "shellexpand",
  "sse-stream",
  "ssh_config",
  "terminal-core",
  "thiserror 2.0.18",
  "tokio",
- "tokio-tungstenite",
  "tokio-util",
  "tracing",
- "urlencoding",
  "uuid",
  "which",
- "x25519-dalek",
 ]
 
 [[package]]
 name = "northhing-test-support"
 version = "0.2.10"
 dependencies = [
  "serde",
  "serde_json",
  "uuid",
 ]
@@ -7851,29 +7830,20 @@ checksum = "d55d956fa96f5ec02be2e13af0e20391a5aa83d6a074e3ad368959d0fab299ea"
 
 [[package]]
 name = "qoi"
 version = "0.4.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "7f6d64c71eb498fe9eae14ce4ec935c555749aef511cca85b5568910d6e48001"
 dependencies = [
  "bytemuck",
 ]
 
-[[package]]
-name = "qrcode"
-version = "0.14.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d68782463e408eb1e668cf6152704bd856c78c5b6417adaee3203d8f4c1fc9ec"
-dependencies = [
- "image",
-]
-
 [[package]]
 name = "quick-error"
 version = "2.0.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "a993555f31e5a609f617c12db6250dedcac1b0a85076912c436e6fc9b2c8e6a3"
 
 [[package]]
 name = "quick-xml"
 version = "0.39.4"
 source = "registry+https://github.com/rust-lang/crates.io-index"
@@ -12966,32 +12936,20 @@ dependencies = [
  "rustix 1.1.4",
  "x11rb-protocol",
 ]
 
 [[package]]
 name = "x11rb-protocol"
 version = "0.13.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "ea6fc2961e4ef194dcbfe56bb845534d0dc8098940c7e5c012a258bfec6701bd"
 
-[[package]]
-name = "x25519-dalek"
-version = "2.0.1"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "c7e468321c81fb07fa7f4c636c3972b9100f0346e5b6a9f2bd0603a52f7ed277"
-dependencies = [
- "curve25519-dalek",
- "rand_core 0.6.4",
- "serde",
- "zeroize",
-]
-
 [[package]]
 name = "xattr"
 version = "1.6.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "32e45ad4206f6d2479085147f02bc2ef834ac85886624a23575ae137c8aa8156"
 dependencies = [
  "libc",
  "rustix 1.1.4",
 ]
 
@@ -13333,34 +13291,20 @@ dependencies = [
  "quote",
  "syn 2.0.118",
  "synstructure",
 ]
 
 [[package]]
 name = "zeroize"
 version = "1.9.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "e13c156562582aa81c60cb29407084cdb54c4164760106ab78e6c5b0858cf64e"
-dependencies = [
- "zeroize_derive",
-]
-
-[[package]]
-name = "zeroize_derive"
-version = "1.5.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "3c50655cbb0fe3fc43170059e702f1ce5e19b84cec58dc87b037a09935c2f328"
-dependencies = [
- "proc-macro2",
- "quote",
- "syn 2.0.118",
-]
 
 [[package]]
 name = "zerotrie"
 version = "0.2.4"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "0f9152d31db0792fa83f70fb2f83148effb5c1f5b8c7686c3459e361d9bc20bf"
 dependencies = [
  "displaydoc",
  "yoke",
  "zerofrom",
diff --git a/scripts/core-boundaries/rules/crate-rules.mjs b/scripts/core-boundaries/rules/crate-rules.mjs
index 19c3b25..a36ba5f 100644
--- a/scripts/core-boundaries/rules/crate-rules.mjs
+++ b/scripts/core-boundaries/rules/crate-rules.mjs
@@ -412,15 +412,14 @@ export const dependencyProfileRules = [
       'futures',
       'git2',
       'notify',
       'rand',
       'reqwest',
       'rmcp',
       'sha2',
       'sse-stream',
       'thiserror',
       'tokio-util',
-      'tokio-tungstenite',
       'uuid',
     ],
   },
 ];
diff --git a/scripts/core-boundaries/rules/feature-rules.mjs b/scripts/core-boundaries/rules/feature-rules.mjs
index 3abcda4..838b505 100644
--- a/scripts/core-boundaries/rules/feature-rules.mjs
+++ b/scripts/core-boundaries/rules/feature-rules.mjs
@@ -36,63 +36,53 @@ export const optionalDependencyFeatureOwnerRules = [
       { depName: 'tokio-tungstenite', ownerFeatures: ['service-integrations'] },
       { depName: 'tower-http', ownerFeatures: ['service-integrations'] },
       { depName: 'tool-runtime', ownerFeatures: ['product-full'] },
     ],
   },
   {
     crateName: 'services-integrations',
     reason:
       'services-integrations optional runtime dependencies must stay owned by explicit integration features',
     dependencies: [
-      { depName: 'aes-gcm', ownerFeatures: ['mcp', 'remote-connect', 'remote-ssh-concrete'] },
-      { depName: 'anyhow', ownerFeatures: ['mcp', 'remote-connect', 'remote-ssh-concrete'] },
+      { depName: 'aes-gcm', ownerFeatures: ['mcp', 'remote-ssh-concrete'] },
+      { depName: 'anyhow', ownerFeatures: ['mcp', 'remote-ssh-concrete'] },
       {
         depName: 'base64',
-        ownerFeatures: ['mcp', 'miniapp-runtime', 'remote-connect', 'remote-ssh-concrete'],
+        ownerFeatures: ['mcp', 'miniapp-runtime', 'remote-ssh-concrete'],
       },
       { depName: 'northhing-product-domains', ownerFeatures: ['function-agents', 'miniapp-runtime'] },
-      { depName: 'northhing-runtime-ports', ownerFeatures: ['remote-connect'] },
+      { depName: 'northhing-runtime-ports', ownerFeatures: ['deep-research'] },
       {
         depName: 'northhing-services-core',
         ownerFeatures: ['git', 'mcp', 'miniapp-runtime', 'workspace-search', 'remote-ssh-concrete'],
       },
-      { depName: 'chrono', ownerFeatures: ['git', 'remote-connect', 'remote-ssh-concrete'] },
+      { depName: 'chrono', ownerFeatures: ['git', 'remote-ssh-concrete'] },
       { depName: 'dirs', ownerFeatures: ['miniapp-runtime', 'remote-ssh-concrete'] },
       { depName: 'dunce', ownerFeatures: ['remote-ssh', 'workspace-search'] },
-      { depName: 'futures', ownerFeatures: ['mcp', 'remote-connect'] },
+      { depName: 'futures', ownerFeatures: ['mcp'] },
       { depName: 'git2', ownerFeatures: ['git'] },
-      { depName: 'hostname', ownerFeatures: ['remote-connect'] },
-      { depName: 'image', ownerFeatures: ['remote-connect'] },
-      { depName: 'mac_address', ownerFeatures: ['remote-connect'] },
       { depName: 'notify', ownerFeatures: ['file-watch'] },
-      { depName: 'qrcode', ownerFeatures: ['remote-connect'] },
-      { depName: 'rand', ownerFeatures: ['mcp', 'remote-connect', 'remote-ssh-concrete'] },
+      { depName: 'rand', ownerFeatures: ['mcp', 'remote-ssh-concrete'] },
       { depName: 'reqwest', ownerFeatures: ['mcp', 'miniapp-runtime'] },
       { depName: 'rmcp', ownerFeatures: ['mcp'] },
       { depName: 'russh', ownerFeatures: ['remote-ssh-concrete'] },
       { depName: 'russh-keys', ownerFeatures: ['remote-ssh-concrete'] },
       { depName: 'russh-sftp', ownerFeatures: ['remote-ssh-concrete'] },
-      { depName: 'rustls', ownerFeatures: ['remote-connect'] },
-      { depName: 'rustls-native-certs', ownerFeatures: ['remote-connect'] },
-      { depName: 'schannel', ownerFeatures: ['remote-connect'] },
-      { depName: 'sha2', ownerFeatures: ['remote-connect', 'remote-ssh'] },
+      { depName: 'sha2', ownerFeatures: ['remote-ssh'] },
       { depName: 'shellexpand', ownerFeatures: ['remote-ssh-concrete'] },
       { depName: 'sse-stream', ownerFeatures: ['mcp'] },
       { depName: 'ssh_config', ownerFeatures: ['remote-ssh-concrete', 'ssh_config'] },
       { depName: 'terminal-core', ownerFeatures: ['remote-ssh-concrete'] },
       { depName: 'thiserror', ownerFeatures: ['git', 'remote-ssh-concrete', 'workspace-search'] },
-      { depName: 'tokio-tungstenite', ownerFeatures: ['remote-connect'] },
       { depName: 'tokio-util', ownerFeatures: ['remote-ssh'] },
-      { depName: 'urlencoding', ownerFeatures: ['remote-connect'] },
-      { depName: 'uuid', ownerFeatures: ['miniapp-runtime', 'remote-connect', 'remote-ssh-concrete'] },
+      { depName: 'uuid', ownerFeatures: ['miniapp-runtime', 'remote-ssh-concrete'] },
       { depName: 'which', ownerFeatures: ['miniapp-runtime', 'workspace-search'] },
-      { depName: 'x25519-dalek', ownerFeatures: ['remote-connect'] },
     ],
   },
   {
     crateName: 'product-domains',
     reason:
       'product-domains optional runtime dependencies must stay owned by explicit product-domain features',
     dependencies: [
       { depName: 'dirs', ownerFeatures: ['miniapp'] },
       { depName: 'sha2', ownerFeatures: ['miniapp'] },
       { depName: 'which', ownerFeatures: ['miniapp'] },
@@ -143,21 +133,20 @@ export const ownerCrateFeatureAssemblyRules = [
     manifestPath: 'src/crates/services/services-integrations/Cargo.toml',
     reason: 'services-integrations must keep integration feature groups explicit and default-light',
     requiredProductFullFeatures: [
       'announcement',
       'deep-research',
       'file-watch',
       'function-agents',
       'git',
       'miniapp-runtime',
       'mcp',
-      'remote-connect',
       'remote-ssh',
       'remote-ssh-concrete',
       'workspace-search',
     ],
   },
   {
     manifestPath: 'src/crates/contracts/product-domains/Cargo.toml',
     reason: 'product-domains must keep product domain feature groups explicit and default-light',
     requiredProductFullFeatures: ['miniapp', 'function-agents'],
   },
diff --git a/scripts/core-boundaries/rules/source/required-rules.mjs b/scripts/core-boundaries/rules/source/required-rules.mjs
index cb5ff45..56389c7 100644
--- a/scripts/core-boundaries/rules/source/required-rules.mjs
+++ b/scripts/core-boundaries/rules/source/required-rules.mjs
@@ -4008,667 +4008,20 @@ export const requiredContentRules = [
     path: 'src/crates/assembly/core/src/agentic/tools/implementations/session_message_tool/sm_resolve.rs',
     reason:
       'SessionMessage must create, resolve, validate, and submit target agent sessions through the service/agent runtime lifecycle owner',
     patterns: [
       {
         regex: /\bAgentDialogPrependedReminder\b/,
         message: 'missing portable prepended reminder request',
       },
     ],
   },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/mod.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bpub mod device\b/,
-        message: 'missing remote-connect device owner module',
-      },
-      {
-        regex: /\bpub mod encryption\b/,
-        message: 'missing remote-connect encryption owner module',
-      },
-      {
-        regex: /\bpub mod pairing\b/,
-        message: 'missing remote-connect pairing owner module',
-      },
-      {
-        regex: /\bpub mod qr_generator\b/,
-        message: 'missing remote-connect QR owner module',
-      },
-      {
-        regex: /\bpub mod relay_client\b/,
-        message: 'missing remote-connect relay client owner module',
-      },
-      {
-        regex: /\bpub use device::DeviceIdentity\b/,
-        message: 'missing remote-connect device compatibility export',
-      },
-      {
-        regex: /\bpub use encryption::\{decrypt_from_base64, encrypt_to_base64, KeyPair\}/,
-        message: 'missing remote-connect encryption compatibility export',
-      },
-      {
-        regex:
-          /pub use pairing::\{[\s\S]*\bPairingChallenge\b[\s\S]*\bPairingProtocol\b[\s\S]*\bPairingResponse\b[\s\S]*\bPairingState\b[\s\S]*\bQrPayload\b[\s\S]*\}/,
-        message: 'missing remote-connect pairing compatibility export',
-      },
-      {
-        regex: /\bpub use qr_generator::QrGenerator\b/,
-        message: 'missing remote-connect QR compatibility export',
-      },
-      {
-        regex:
-          /pub use relay_client::\{[\s\S]*\bConnectionState\b[\s\S]*\bRelayClient\b[\s\S]*\bRelayEvent\b[\s\S]*\bRelayMessage\b[\s\S]*\}/,
-        message: 'missing remote-connect relay compatibility export',
-      },
-      {
-        regex: /\bpub trait RemoteCommandRuntimeHost\b/,
-        message: 'missing remote command runtime host contract',
-      },
-      {
-        regex: /\bpub async fn handle_remote_command\b/,
-        message: 'missing remote command routing owner',
-      },
-      {
-        regex: /\bpub enum RemoteCommand\b/,
-        message: 'missing remote command wire contract',
-      },
-      {
-        regex: /\bpub enum RemoteResponse\b/,
-        message: 'missing remote response wire contract',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_session_state.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bpub struct RemoteSessionStateTracker\b/,
-        message: 'missing remote session state tracker owner',
-      },
-      {
-        regex: /\bpub enum TrackerEvent\b/,
-        message: 'missing remote tracker event owner',
-      },
-      {
-        regex: /\bpub trait RemoteSessionTrackerHost\b/,
-        message: 'missing remote tracker host port',
-      },
-      {
-        regex: /\bpub struct RemoteSessionTrackerRegistry\b/,
-        message: 'missing remote tracker registry owner',
-      },
-      {
-        regex: /\bfn handle_agentic_event\b/,
-        message: 'missing tracker event reducer',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_request_builders.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bpub fn make_slim_tool_params\b/,
-        message: 'missing remote tool preview slimming helper',
-      },
-      {
-        regex: /\bpub struct RemoteImageContext\b/,
-        message: 'missing portable remote image context contract',
-      },
-      {
-        regex: /\bpub trait RemoteImageContextAdapter\b/,
-        message: 'missing remote image context adapter contract',
-      },
-      {
-        regex: /\bpub fn build_remote_image_contexts\b/,
-        message: 'missing legacy remote image context builder',
-      },
-      {
-        regex: /\bpub fn resolve_remote_execution_image_contexts\b/,
-        message: 'missing remote image context preference helper',
-      },
-      {
-        regex: /\bpub fn remote_session_restore_target\b/,
-        message: 'missing remote restore-target helper',
-      },
-      {
-        regex: /\bpub struct RemoteChatHistoryTurn\b/,
-        message: 'missing remote chat history turn DTO',
-      },
-      {
-        regex: /\bpub struct RemoteChatHistoryRound\b/,
-        message: 'missing remote chat history round DTO',
-      },
-      {
-        regex: /\bpub struct RemoteChatHistoryToolItem\b/,
-        message: 'missing remote chat history tool item DTO',
-      },
-      {
-        regex: /\bpub fn build_remote_chat_messages\b/,
-        message: 'missing remote chat history assembly owner',
-      },
-      {
-        regex: /\bpub struct RemoteDefaultModelsConfig\b/,
-        message: 'missing remote model default DTO',
-      },
-      {
-        regex: /\bpub struct RemoteModelConfig\b/,
-        message: 'missing remote model DTO',
-      },
-      {
-        regex: /\bpub struct RemoteModelCatalog\b/,
-        message: 'missing remote model catalog DTO',
-      },
-      {
-        regex: /\bpub enum RemoteModelCapabilityFact\b/,
-        message: 'missing remote model capability owner fact',
-      },
-      {
-        regex: /\bpub enum RemoteReasoningModeFact\b/,
-        message: 'missing remote reasoning mode owner fact',
-      },
-      {
-        regex: /\bpub struct RemoteModelFacts\b/,
-        message: 'missing remote model owner facts',
-      },
-      {
-        regex: /\bpub struct RemoteModelCatalogFacts\b/,
-        message: 'missing remote model catalog owner facts',
-      },
-      {
-        regex: /\bpub fn build_remote_model_catalog\b/,
-        message: 'missing remote model catalog assembly owner',
-      },
-      {
-        regex: /\bpub struct RemoteModelCatalogPollDelta\b/,
-        message: 'missing remote model catalog poll delta',
-      },
-      {
-        regex: /\bpub fn normalize_remote_session_model_id\b/,
-        message: 'missing remote session model normalization policy',
-      },
-      {
-        regex: /\bpub fn normalize_remote_model_selection\b/,
-        message: 'missing remote model selection policy',
-      },
-      {
-        regex: /\bpub fn remote_model_selection_needs_config\b/,
-        message: 'missing remote model selection config-gate policy',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_workspace_resolver.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bpub fn resolve_remote_agent_type\b/,
-        message: 'missing remote agent type helper',
-      },
-      {
-        regex: /\bpub const REMOTE_FILE_MAX_READ_BYTES\b/,
-        message: 'missing remote file max-read policy',
-      },
-      {
-        regex: /\bpub const REMOTE_FILE_MAX_CHUNK_BYTES\b/,
-        message: 'missing remote file chunk policy',
-      },
-      {
-        regex: /\bpub fn resolve_remote_file_chunk_range\b/,
-        message: 'missing remote file chunk range helper',
-      },
-      {
-        regex: /\bpub fn resolve_remote_workspace_path\b/,
-        message: 'missing remote workspace path resolver',
-      },
-      {
-        regex: /\bpub fn detect_remote_mime_type\b/,
-        message: 'missing remote MIME detector',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_cancel_handlers.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bpub enum RemoteCancelDecision\b/,
-        message: 'missing remote cancel decision contract',
-      },
-      {
-        regex: /\bpub fn resolve_remote_cancel_decision\b/,
-        message: 'missing remote cancel decision resolver',
-      },
-      {
-        regex: /\bpub struct RemoteCancelTaskRequest\b/,
-        message: 'missing remote cancel task request contract',
-      },
-      {
-        regex: /\bpub trait RemoteCancelRuntimeHost\b/,
-        message: 'missing remote cancel runtime host port',
-      },
-      {
-        regex: /\bpub async fn cancel_remote_task\b/,
-        message: 'missing remote cancel orchestration owner',
-      },
-      {
-        regex: /\bpub fn remote_task_cancel_response\b/,
-        message: 'missing remote task cancel response assembly helper',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_dialog_handlers.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bpub trait RemoteDialogRuntimeHost\b/,
-        message: 'missing remote dialog runtime host port',
-      },
-      {
-        regex: /\bpub async fn submit_remote_dialog\b/,
-        message: 'missing remote dialog orchestration owner',
-      },
-      {
-        regex: /\bpub fn remote_dialog_submit_response\b/,
-        message: 'missing remote dialog response assembly helper',
-      },
-      {
-        regex: /\bpub enum RemoteDialogSchedulerOutcomeFact\b/,
-        message: 'missing remote dialog scheduler outcome fact',
-      },
-      {
-        regex: /\bpub fn remote_dialog_submit_outcome_from_scheduler\b/,
-        message: 'missing remote dialog submit outcome assembly owner',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_file_io.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bpub fn remote_file_display_name\b/,
-        message: 'missing remote file display-name fallback',
-      },
-      {
-        regex: /\bpub async fn read_remote_workspace_file\b/,
-        message: 'missing remote workspace full-file reader',
-      },
-      {
-        regex: /\bpub async fn read_remote_workspace_file_chunk\b/,
-        message: 'missing remote workspace chunk reader',
-      },
-      {
-        regex: /\bpub async fn read_remote_workspace_file_info\b/,
-        message: 'missing remote workspace file-info reader',
-      },
-      {
-        regex: /\bpub fn remote_file_content_response\b/,
-        message: 'missing remote file content response assembly helper',
-      },
-      {
-        regex: /\bpub fn remote_file_chunk_response\b/,
-        message: 'missing remote file chunk response assembly helper',
-      },
-      {
-        regex: /\bpub fn remote_file_info_response\b/,
-        message: 'missing remote file-info response assembly helper',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_session_handlers.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bRemoteWorkspaceFileRuntimeHost\b/,
-        message: 'missing remote workspace file runtime host contract',
-      },
-      {
-        regex: /\bRemoteWorkspaceRuntimeHost\b/,
-        message: 'missing remote workspace runtime host contract',
-      },
-      {
-        regex: /\bRemoteSessionRuntimeHost\b/,
-        message: 'missing remote session runtime host contract',
-      },
-      {
-        regex: /\bRemotePollRuntimeHost\b/,
-        message: 'missing remote poll runtime host contract',
-      },
-      {
-        regex: /\bRemoteInteractionRuntimeHost\b/,
-        message: 'missing remote interaction runtime host contract',
-      },
-      {
-        regex: /\bpub async fn handle_remote_workspace_file_command\b/,
-        message: 'missing remote workspace file command owner handler',
-      },
-      {
-        regex: /\bpub async fn handle_remote_workspace_command\b/,
-        message: 'missing remote workspace command owner handler',
-      },
-      {
-        regex: /\bpub async fn handle_remote_interaction_command\b/,
-        message: 'missing remote interaction command owner handler',
-      },
-      {
-        regex: /\bpub fn remote_interaction_accepted_response\b/,
-        message: 'missing remote interaction response assembly helper',
-      },
-      {
-        regex: /\bpub fn remote_answer_question_response\b/,
-        message: 'missing remote answer response assembly helper',
-      },
-      {
-        regex: /\bRemoteWorkspaceFacts\b/,
-        message: 'missing remote workspace response facts DTO',
-      },
-      {
-        regex: /\bRemoteSessionMetadata\b/,
-        message: 'missing remote session response metadata DTO',
-      },
-      {
-        regex: /\bpub fn remote_workspace_info_response\b/,
-        message: 'missing remote workspace-info response assembly helper',
-      },
-      {
-        regex: /\bpub fn remote_recent_workspaces_response\b/,
-        message: 'missing remote recent-workspaces response assembly helper',
-      },
-      {
-        regex: /\bpub fn remote_assistant_list_response\b/,
-        message: 'missing remote assistant-list response assembly helper',
-      },
-      {
-        regex: /\bremote_workspace_handler_preserves_response_shapes\b/,
-        message: 'missing remote workspace command handler regression',
-      },
-      {
-        regex: /\bremote_session_handler_preserves_list_and_create_policy\b/,
-        message: 'missing remote session command handler regression',
-      },
-      {
-        regex: /\bremote_session_handler_removes_tracker_after_delete_success\b/,
-        message: 'missing remote session delete tracker cleanup regression',
-      },
-      {
-        regex: /\bremote_poll_handler_preserves_missing_workspace_error\b/,
-        message: 'missing remote poll missing-workspace regression',
-      },
-      {
-        regex: /\bremote_interaction_handler_preserves_default_reject_reason\b/,
-        message: 'missing remote interaction default reject regression',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/src/remote_connect/remote_session_response_builders.rs',
-    reason:
-      'services-integrations must own remote-connect wire/response assembly and preserve remote owner compatibility re-exports',
-    patterns: [
-      {
-        regex: /\bRemoteInitialSyncRuntimeHost\b/,
-        message: 'missing remote initial-sync runtime host contract',
-      },
-      {
-        regex: /\bpub async fn generate_remote_initial_sync\b/,
-        message: 'missing remote initial-sync owner handler',
-      },
-      {
-        regex: /\bpub async fn handle_remote_session_command\b/,
-        message: 'missing remote session command owner handler',
-      },
-      {
-        regex: /\bpub async fn handle_remote_poll_command\b/,
-        message: 'missing remote poll command owner handler',
-      },
-      {
-        regex: /\bpub fn remote_session_info\b/,
-        message: 'missing remote session response facts helper',
-      },
-      {
-        regex: /\bpub fn remote_session_list_response\b/,
-        message: 'missing remote session-list response assembly helper',
-      },
-      {
-        regex: /\bpub fn remote_initial_sync_response\b/,
-        message: 'missing remote initial-sync response assembly helper',
-      },
-      {
-        regex: /\bpub fn remote_messages_response\b/,
-        message: 'missing remote messages response assembly helper',
-      },
-      {
-        regex: /\bpub fn should_send_remote_model_catalog\b/,
-        message: 'missing remote model catalog poll policy',
-      },
-      {
-        regex: /\bpub fn remote_model_catalog_poll_delta\b/,
-        message: 'missing remote model catalog poll delta helper',
-      },
-      {
-        regex: /\bpub fn remote_no_change_poll_response\b/,
-        message: 'missing remote no-change poll response helper',
-      },
-      {
-        regex: /\bpub fn remote_snapshot_poll_response\b/,
-        message: 'missing remote snapshot poll response helper',
-      },
-      {
-        regex: /\bpub fn remote_persisted_poll_response\b/,
-        message: 'missing remote persisted poll response helper',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/tests/pairing_qr_relay.rs',
-    reason: 'remote-connect owner crate must keep focused behavior contracts',
-    patterns: [
-      {
-        regex: /\bremote_connect_pairing_primitives_live_in_services_owner\b/,
-        message: 'missing remote-connect pairing/encryption owner contract test',
-      },
-      {
-        regex: /\bremote_connect_qr_and_relay_primitives_live_in_services_owner\b/,
-        message: 'missing remote-connect QR/relay owner contract test',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/tests/session_wire_and_responses.rs',
-    reason: 'remote-connect owner crate must keep focused behavior contracts',
-    patterns: [
-      {
-        regex: /\bremote_connect_command_wire_shape_lives_in_owner_contract\b/,
-        message: 'missing remote command wire contract test',
-      },
-      {
-        regex: /\bremote_connect_response_wire_shape_lives_in_owner_contract\b/,
-        message: 'missing remote response wire contract test',
-      },
-      {
-        regex: /\bremote_connect_execution_response_helpers_preserve_wire_shape\b/,
-        message: 'missing remote execution response helper contract test',
-      },
-      {
-        regex: /\bremote_connect_workspace_response_helpers_own_wire_shape\b/,
-        message: 'missing remote workspace response assembly regression',
-      },
-      {
-        regex: /\bremote_connect_session_response_helpers_own_pagination_and_timestamps\b/,
-        message: 'missing remote session response assembly regression',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/tests/model_catalog_tracker_poll.rs',
-    reason: 'remote-connect owner crate must keep focused behavior contracts',
-    patterns: [
-      {
-        regex: /\bremote_connect_model_catalog_delta_preserves_poll_invalidation_policy\b/,
-        message: 'missing remote model catalog delta contract test',
-      },
-      {
-        regex: /\bremote_connect_model_catalog_builder_preserves_config_shape\b/,
-        message: 'missing remote model catalog builder contract test',
-      },
-      {
-        regex: /\bremote_connect_model_selection_policy_owns_alias_and_config_reference_rules\b/,
-        message: 'missing remote model selection policy contract test',
-      },
-      {
-        regex: /\bremote_connect_poll_helpers_preserve_delta_and_completion_policy\b/,
-        message: 'missing remote poll helper contract test',
-      },
-      {
-        regex: /\bremote_connect_tracker_keeps_finished_turn_snapshot_until_persistence_finalizes\b/,
-        message: 'missing tracker completion contract test',
-      },
-      {
-        regex: /\bremote_connect_tracker_registry_owns_lifecycle_without_core_state\b/,
-        message: 'missing tracker registry owner test',
-      },
-      {
-        regex: /\bremote_connect_tracker_ignores_unrelated_direct_session_events\b/,
-        message: 'missing tracker unrelated-event guard test',
-      },
-      {
-        regex: /\bremote_connect_tool_preview_slimming_keeps_short_fields_and_drops_large_strings\b/,
-        message: 'missing remote tool preview slimming test',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/tests/submission_images.rs',
-    reason: 'remote-connect owner crate must keep focused behavior contracts',
-    patterns: [
-      {
-        regex: /\bremote_connect_image_context_policy_preserves_legacy_fallback_shape\b/,
-        message: 'missing legacy image context fallback test',
-      },
-      {
-        regex: /\bremote_connect_image_context_policy_prefers_explicit_contexts\b/,
-        message: 'missing explicit image context preference test',
-      },
-      {
-        regex: /\bremote_connect_image_context_adapter_owns_portable_conversion_shape\b/,
-        message: 'missing image context adapter contract test',
-      },
-      {
-        regex: /\bremote_chat_history_assembly_preserves_message_shape_and_item_order\b/,
-        message: 'missing remote chat history assembly shape/order test',
-      },
-      {
-        regex: /\bremote_chat_history_assembly_skips_in_progress_assistant_history\b/,
-        message: 'missing remote chat history in-progress guard test',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/tests/dialog_cancel_contracts.rs',
-    reason: 'remote-connect owner crate must keep focused behavior contracts',
-    patterns: [
-      {
-        regex: /\bremote_connect_cancel_and_restore_policy_preserve_runtime_decisions\b/,
-        message: 'missing cancel/restore policy test',
-      },
-      {
-        regex: /\bremote_connect_dialog_runtime_owns_restore_prewarm_and_submit_order\b/,
-        message: 'missing dialog runtime order test',
-      },
-      {
-        regex: /\bremote_connect_dialog_runtime_preserves_explicit_turn_without_restore\b/,
-        message: 'missing dialog explicit-turn test',
-      },
-      {
-        regex: /\bremote_connect_dialog_submit_outcome_builder_preserves_scheduler_shape\b/,
-        message: 'missing remote dialog outcome builder contract test',
-      },
-      {
-        regex: /\bremote_connect_dialog_runtime_keeps_legacy_restore_failure_tolerance\b/,
-        message: 'missing restore failure tolerance test',
-      },
-      {
-        regex: /\bremote_connect_cancel_runtime_restores_missing_session_before_cancel\b/,
-        message: 'missing remote cancel restore/order regression',
-      },
-      {
-        regex: /\bremote_connect_cancel_runtime_preserves_stale_and_idle_errors_without_restore\b/,
-        message: 'missing remote cancel stale/idle regression',
-      },
-      {
-        regex: /\bremote_connect_cancel_runtime_preserves_restore_failure_error\b/,
-        message: 'missing remote cancel restore failure regression',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/tests/file_transfer.rs',
-    reason: 'remote-connect owner crate must keep focused behavior contracts',
-    patterns: [
-      {
-        regex: /\bremote_connect_file_transfer_policy_preserves_limits_and_chunk_ranges\b/,
-        message: 'missing remote file transfer policy test',
-      },
-      {
-        regex: /\bremote_connect_file_transfer_policy_preserves_name_fallback\b/,
-        message: 'missing remote file display-name test',
-      },
-      {
-        regex: /\bremote_connect_file_path_resolution_stays_within_workspace_root\b/,
-        message: 'missing remote file path resolution test',
-      },
-      {
-        regex: /\bremote_connect_file_read_helpers_preserve_current_wire_inputs\b/,
-        message: 'missing remote full-read helper test',
-      },
-      {
-        regex: /\bremote_connect_file_chunk_and_info_helpers_preserve_response_facts\b/,
-        message: 'missing remote chunk/info helper test',
-      },
-      {
-        regex: /\bremote_connect_file_response_assembly_owns_base64_wire_shape\b/,
-        message: 'missing remote file response assembly contract test',
-      },
-      {
-        regex: /\bremote_connect_file_command_handler_owns_owner_flow_and_uses_host_root\b/,
-        message: 'missing remote file command handler owner-flow test',
-      },
-    ],
-  },
-  {
-    path: 'src/crates/services/services-integrations/tests/command_runtime.rs',
-    reason: 'remote-connect owner crate must keep focused behavior contracts',
-    patterns: [
-      {
-        regex: /\bremote_connect_command_owner_routes_send_message_and_prefers_explicit_images\b/,
-        message: 'missing remote command routing image/source regression',
-      },
-      {
-        regex: /\bremote_connect_command_owner_preserves_cancel_and_group_routing\b/,
-        message: 'missing remote command routing group/cancel regression',
-      },
-    ],
-  },
   {
     path: 'src/crates/assembly/core/src/agentic/coordination/scheduler/scheduler_turn/turn_submit.rs',
     reason:
       'core scheduler keeps remote queue policy semantics until agent-runtime migration is reviewed',
     patterns: [
       {
         regex: /\bimpl AgentDialogTurnPort for DialogScheduler\b/,
         message: 'missing dialog lifecycle port implementation',
       },
       {
diff --git a/scripts/core-boundaries/self-test.mjs b/scripts/core-boundaries/self-test.mjs
index abddab0..87c73c8 100644
--- a/scripts/core-boundaries/self-test.mjs
+++ b/scripts/core-boundaries/self-test.mjs
@@ -562,27 +562,22 @@ export function runManifestParserSelfTest({
   );
   if (!coreGit2Owner?.ownerFeatures.includes('service-integrations')) {
     throw new Error('core optional dependency owner rule must keep git2 under service-integrations');
   }
   const servicesOptionalOwnerRule = optionalDependencyFeatureOwnerRules.find(
     (rule) => rule.crateName === 'services-integrations',
   );
   for (const dep of [
     'northhing-runtime-ports',
     'git2',
-    'hostname',
-    'mac_address',
     'notify',
-    'qrcode',
     'rmcp',
-    'tokio-tungstenite',
-    'x25519-dalek',
   ]) {
     if (!servicesOptionalOwnerRule?.dependencies.some((dependency) => dependency.depName === dep)) {
       throw new Error(`services-integrations optional dependency owner rule must cover ${dep}`);
     }
   }
   const productDomainsOptionalOwnerRule = optionalDependencyFeatureOwnerRules.find(
     (rule) => rule.crateName === 'product-domains',
   );
   for (const dep of ['dirs', 'sha2']) {
     if (!productDomainsOptionalOwnerRule?.dependencies.some((dependency) => dependency.depName === dep)) {
@@ -1680,106 +1675,20 @@ export function runManifestParserSelfTest({
         'AgentSessionListRequest',
         'AgentSessionWorkspaceRequest',
         'list_sessions',
         'resolve_session_workspace_path',
         '"createdBy"',
         'AgentDialogTurnRequest',
         'AgentDialogPrependedReminder',
         'submit_dialog_turn',
       ],
     },
-    {
-      path: 'src/crates/services/services-integrations/src/remote_connect.rs',
-      contracts: [
-        'pub mod device',
-        'pub mod encryption',
-        'pub mod pairing',
-        'pub mod qr_generator',
-        'pub mod relay_client',
-        'pub use device::DeviceIdentity',
-        'pub use encryption::{decrypt_from_base64, encrypt_to_base64, KeyPair}',
-        'PairingProtocol',
-        'QrPayload',
-        'pub use qr_generator::QrGenerator',
-        'RelayClient',
-        'RelayMessage',
-        'RemoteSessionStateTracker',
-        'TrackerEvent',
-        'RemoteSessionTrackerHost',
-        'RemoteSessionTrackerRegistry',
-        'make_slim_tool_params',
-        'handle_agentic_event',
-        'resolve_remote_agent_type',
-        'RemoteImageContext',
-        'build_remote_image_contexts',
-        'resolve_remote_execution_image_contexts',
-        'remote_session_restore_target',
-        'RemoteCancelDecision',
-        'resolve_remote_cancel_decision',
-        'RemoteCancelTaskRequest',
-        'RemoteCancelRuntimeHost',
-        'cancel_remote_task',
-        'RemoteChatHistoryTurn',
-        'RemoteChatHistoryRound',
-        'RemoteChatHistoryToolItem',
-        'build_remote_chat_messages',
-        'REMOTE_FILE_MAX_READ_BYTES',
-        'REMOTE_FILE_MAX_CHUNK_BYTES',
-        'resolve_remote_file_chunk_range',
-        'remote_file_display_name',
-        'RemoteWorkspaceFacts',
-        'RemoteSessionMetadata',
-        'remote_workspace_info_response',
-        'remote_recent_workspaces_response',
-        'remote_assistant_list_response',
-        'RemoteWorkspaceRuntimeHost',
-        'handle_remote_workspace_command',
-        'remote_workspace_handler_preserves_response_shapes',
-        'RemoteInitialSyncRuntimeHost',
-        'generate_remote_initial_sync',
-        'remote_session_info',
-        'remote_session_list_response',
-        'remote_initial_sync_response',
-        'remote_messages_response',
-        'RemoteSessionRuntimeHost',
-        'handle_remote_session_command',
-        'remote_session_handler_preserves_list_and_create_policy',
-        'remote_session_handler_removes_tracker_after_delete_success',
-        'RemotePollRuntimeHost',
-        'handle_remote_poll_command',
-        'remote_poll_handler_preserves_missing_workspace_error',
-        'RemoteInteractionRuntimeHost',
-        'handle_remote_interaction_command',
-        'remote_interaction_handler_preserves_default_reject_reason',
-        'RemoteDefaultModelsConfig',
-        'RemoteModelConfig',
-        'RemoteModelCatalog',
-        'RemoteModelCapabilityFact',
-        'RemoteReasoningModeFact',
-        'RemoteModelFacts',
-        'RemoteModelCatalogFacts',
-        'build_remote_model_catalog',
-        'RemoteModelCatalogPollDelta',
-        'normalize_remote_session_model_id',
-        'normalize_remote_model_selection',
-        'remote_model_selection_needs_config',
-        'RemoteDialogSchedulerOutcomeFact',
-        'remote_dialog_submit_outcome_from_scheduler',
-        'RemoteCommand',
-        'RemoteResponse',
-        'should_send_remote_model_catalog',
-        'remote_model_catalog_poll_delta',
-        'remote_no_change_poll_response',
-        'remote_snapshot_poll_response',
-        'remote_persisted_poll_response',
-      ],
-    },
     {
       path: 'src/crates/assembly/core/src/agentic/coordination/scheduler.rs',
       contracts: [
         'remote_queue_policy_preserves_confirmation_boundary',
         'AgentDialogTurnPort',
         'AgentLifecycleDeliveryPort',
         'AgentTurnCancellationPort',
         'AgentBackgroundResultRequest',
         'AgentThreadGoalDeliveryRequest',
         'AgentThreadGoalDeliveryKind::ObjectiveUpdated',
diff --git a/src/crates/services/AGENTS-CN.md b/src/crates/services/AGENTS-CN.md
index 8b70327..7c41ccd 100644
--- a/src/crates/services/AGENTS-CN.md
+++ b/src/crates/services/AGENTS-CN.md
@@ -2,21 +2,21 @@
 
 # 服务实现层
 
 本层负责接触本地系统或 runtime infrastructure 的可复用具体实现：filesystem、git、file watch、terminal、MCP、remote connectivity、process lifecycle、session persistence primitives、MiniApp runtime/import IO 以及类似 OS/network 能力。
 
 ## 模块
 
 | Crate | 职责 | 本地文档 |
 |---|---|---|
 | `services-core` | 不包含产品组装决策的本地 service primitive，包括 session storage、metadata store CRUD/index rebuild、metadata 构造/计数/索引/字段 mutation、lineage 规则和 JSON file IO | [AGENTS.md](services-core/AGENTS.md) |
-| `services-integrations` | MCP、git、remote、file watch、MiniApp runtime、产品领域 port 具体实现，以及平台无关的 Remote Connect primitives | [AGENTS.md](services-integrations/AGENTS.md) |
+| `services-integrations` | MCP、git、remote、file watch、MiniApp runtime、产品领域 port 具体实现 | [AGENTS.md](services-integrations/AGENTS.md) |
 | `terminal` | PTY、shell integration 与 terminal session infrastructure | [AGENTS.md](terminal/AGENTS.md) |
 
 ## 放置规则
 
 - 具体 OS、process、filesystem、git、terminal、MCP、remote SSH、file watch、session persistence primitives、MiniApp runtime IO 和 network service 实现放在这里。
 - 需要具体依赖的 `contracts`、`execution` 或 `contracts/product-domains` port 实现在这里。
 - 协议/transport projection 放在 `adapters`，产品能力选择放在 `assembly`。
 
 ## 依赖边界
 
diff --git a/src/crates/services/AGENTS.md b/src/crates/services/AGENTS.md
index 439c668..9003593 100644
--- a/src/crates/services/AGENTS.md
+++ b/src/crates/services/AGENTS.md
@@ -5,21 +5,21 @@
 This layer owns reusable concrete implementations that touch local systems or
 runtime infrastructure: filesystem, git, file watch, terminal, MCP, remote
 connectivity, process lifecycle, session persistence primitives, MiniApp concrete runtime IO, and similar
 OS/network capabilities.
 
 ## Modules
 
 | Crate | Responsibility | Local doc |
 |---|---|---|
 | `services-core` | Reusable local service primitives, filesystem helpers, session storage layout/indexing/deletion, metadata store CRUD/index rebuild, metadata construction/counter/index/field mutation/lineage rules, and JSON file IO without product assembly decisions | [AGENTS.md](services-core/AGENTS.md) |
-| `services-integrations` | Concrete MCP, git, remote, file-watch, MiniApp runtime, product-domain port implementations, and platform-neutral Remote Connect primitives | [AGENTS.md](services-integrations/AGENTS.md) |
+| `services-integrations` | Concrete MCP, git, remote, file-watch, MiniApp runtime, and product-domain port implementations | [AGENTS.md](services-integrations/AGENTS.md) |
 | `terminal` | PTY, shell integration, and terminal session infrastructure | [AGENTS.md](terminal/AGENTS.md) |
 | `debug-log` | Debug-mode runtime logging leaf crate (`log_event` + `COMP_*` component constants and the disk-append pipeline); shared by product surfaces and re-exported from `assembly/core` | (none) |
 
 ## Placement Rules
 
 - Put concrete OS, process, filesystem, git, terminal, MCP, remote SSH,
   file-watch, MiniApp runtime IO, and network service implementations here.
 - Implement `contracts`, `execution`, or `contracts/product-domains` ports here
   when the implementation needs concrete dependencies.
 - Keep protocol/transport projection in `adapters`, and keep product capability
diff --git a/src/crates/services/services-integrations/AGENTS.md b/src/crates/services/services-integrations/AGENTS.md
index 0ac77ca..0b130dc 100644
--- a/src/crates/services/services-integrations/AGENTS.md
+++ b/src/crates/services/services-integrations/AGENTS.md
@@ -10,24 +10,20 @@ slices that are outside pure product logic but still platform-neutral.
 - Do not depend on `northhing-core`, app crates, desktop adapters, CLI UI, or web
   presentation code.
 - Keep integration families behind explicit features. The default feature set
   should not compile heavy Git, MCP, SSH, network, or file-watch runtimes.
   Boundary checks enforce `default = []` and the current `product-full`
   integration feature-group list.
 - MCP config/process/transport lifecycle and dynamic provider helpers may live
   here; product tool registry assembly, manifest filtering, `GetToolSpec`
   execution, and concrete tool behavior remain outside this crate unless a
   reviewed owner move proves behavior equivalence.
-- Remote-connect platform-neutral primitives belong here: device identity,
-  pairing/encryption, QR payload generation, relay client protocol, dialog/cancel
-  orchestration ports, image-context adapter contracts, remote workspace helpers,
-  and command/response assembly.
 - Remote workspace facts, session metadata, file projection DTOs, and
   workspace/projection host traits belong in `northhing-runtime-ports`.
 - Workspace-root source selection, persistence/workspace service reads,
   concrete scheduler/session restore, terminal pre-warm adapters, and product
   execution remain core-owned unless a reviewed port/provider moves them with
   equivalence tests.
 - Remote-SSH path/session identity helpers, SSH channels, SFTP, remote FS,
   remote terminal, and manager assembly live here behind explicit remote SSH
   features.
 - Workspace search owns the local flashgrep daemon/session lifecycle and
diff --git a/src/crates/services/services-integrations/Cargo.toml b/src/crates/services/services-integrations/Cargo.toml
index 93731fe..6894286 100644
--- a/src/crates/services/services-integrations/Cargo.toml
+++ b/src/crates/services/services-integrations/Cargo.toml
@@ -19,53 +19,41 @@ northhing-product-domains = { path = "../../contracts/product-domains", default-
 northhing-runtime-ports = { path = "../../contracts/runtime-ports", optional = true }
 aes-gcm = { workspace = true, optional = true }
 anyhow = { workspace = true, optional = true }
 async-trait = { workspace = true, optional = true }
 base64 = { workspace = true, optional = true }
 northhing-services-core = { path = "../services-core", optional = true }
 chrono = { workspace = true, optional = true }
 dunce = { workspace = true, optional = true }
 futures = { workspace = true, optional = true }
 git2 = { workspace = true, optional = true }
-hostname = { workspace = true, optional = true }
-image = { workspace = true, optional = true }
-mac_address = { workspace = true, optional = true }
 notify = { workspace = true, optional = true }
-qrcode = { workspace = true, optional = true }
 rand = { workspace = true, optional = true }
 reqwest = { workspace = true, optional = true }
 rmcp = { workspace = true, optional = true }
-rustls = { workspace = true, optional = true }
-rustls-native-certs = { version = "0.8", optional = true }
 sha2 = { workspace = true, optional = true }
 sse-stream = { workspace = true, optional = true }
 thiserror = { workspace = true, optional = true }
 tokio-util = { workspace = true, optional = true }
-tokio-tungstenite = { workspace = true, optional = true }
-urlencoding = { workspace = true, optional = true }
 uuid = { workspace = true, optional = true }
 which = { workspace = true, optional = true }
 dirs = { workspace = true, optional = true }
 russh = { workspace = true, optional = true }
 russh-sftp = { workspace = true, optional = true }
 russh-keys = { workspace = true, optional = true }
 shellexpand = { workspace = true, optional = true }
 ssh_config = { workspace = true, optional = true }
 terminal-core = { path = "../terminal", optional = true }
-x25519-dalek = { workspace = true, optional = true }
 
 [target.'cfg(not(windows))'.dependencies]
 git2 = { workspace = true, features = ["vendored-openssl"], optional = true }
 
-[target.'cfg(windows)'.dependencies]
-schannel = { version = "0.1", optional = true }
-
 [dev-dependencies]
 async-trait = { workspace = true }
 northhing-test-support = { path = "../../support/test-support" }
 
 [features]
 default = []
 announcement = []
 deep-research = ["northhing-runtime-ports"]
 git = ["northhing-services-core", "chrono", "git2", "thiserror"]
 file-watch = ["notify"]
@@ -90,42 +78,20 @@ mcp = [
 miniapp-runtime = [
     "base64",
     "northhing-product-domains/miniapp",
     "northhing-services-core",
     "dep:northhing-product-domains",
     "dirs",
     "reqwest",
     "uuid",
     "which",
 ]
-remote-connect = [
-    "anyhow",
-    "aes-gcm",
-    "async-trait",
-    "base64",
-    "northhing-runtime-ports",
-    "chrono",
-    "futures",
-    "hostname",
-    "image",
-    "mac_address",
-    "qrcode",
-    "rand",
-    "rustls",
-    "rustls-native-certs",
-    "schannel",
-    "sha2",
-    "tokio-tungstenite",
-    "urlencoding",
-    "uuid",
-    "x25519-dalek",
-]
 remote-ssh = ["dunce", "sha2", "tokio-util"]
 remote-ssh-concrete = [
     "remote-ssh",
     "northhing-services-core",
     "aes-gcm",
     "anyhow",
     "async-trait",
     "base64",
     "chrono",
     "dirs",
@@ -147,16 +113,15 @@ workspace-search = [
     "which",
 ]
 product-full = [
     "announcement",
     "deep-research",
     "file-watch",
     "function-agents",
     "git",
     "miniapp-runtime",
     "mcp",
-    "remote-connect",
     "remote-ssh",
     "remote-ssh-concrete",
     "workspace-search",
 ]
 ssh_config = ["dep:ssh_config"]
diff --git a/src/crates/services/services-integrations/src/lib.rs b/src/crates/services/services-integrations/src/lib.rs
index d247504..7a847fa 100644
--- a/src/crates/services/services-integrations/src/lib.rs
+++ b/src/crates/services/services-integrations/src/lib.rs
@@ -20,22 +20,19 @@ pub mod function_agents;
 
 #[cfg(feature = "git")]
 pub mod git;
 
 #[cfg(feature = "mcp")]
 pub mod mcp;
 
 #[cfg(feature = "miniapp-runtime")]
 pub mod miniapp;
 
-#[cfg(feature = "remote-connect")]
-pub mod remote_connect;
-
 #[cfg(feature = "remote-ssh")]
 pub mod remote_ssh;
 
 #[cfg(feature = "workspace-search")]
 pub mod workspace_search;
 
 #[cfg(all(windows, feature = "git"))]
 #[link(name = "advapi32")]
 unsafe extern "system" {}
diff --git a/src/crates/services/services-integrations/tests/common/mod.rs b/src/crates/services/services-integrations/tests/common/mod.rs
index 84f6eeb..48fb275 100644
--- a/src/crates/services/services-integrations/tests/common/mod.rs
+++ b/src/crates/services/services-integrations/tests/common/mod.rs
@@ -1,539 +1,17 @@
+#![allow(dead_code, unused_imports)]
+
 pub use async_trait::async_trait;
-pub use northhing_events::{AgenticEvent, ToolEventData};
-pub use northhing_runtime_ports::{AgentSubmissionSource, RemoteControlSessionState, RemoteControlStateSnapshot};
-pub use northhing_services_integrations::remote_connect::{
-    build_remote_chat_messages, build_remote_image_attachment, build_remote_image_contexts,
-    build_remote_image_submission_request, build_remote_model_catalog, build_remote_session_create_request,
-    build_remote_submission_request, cancel_remote_task, handle_remote_command, handle_remote_workspace_file_command,
-    make_slim_tool_params, normalize_remote_model_selection, normalize_remote_session_model_id,
-    read_remote_workspace_file, read_remote_workspace_file_chunk, read_remote_workspace_file_info,
-    remote_answer_question_response, remote_assistant_list_response, remote_assistant_updated_response,
-    remote_dialog_submit_outcome_from_scheduler, remote_dialog_submit_response, remote_file_chunk_response,
-    remote_file_content_response, remote_file_display_name, remote_file_info_response, remote_initial_sync_response,
-    remote_interaction_accepted_response, remote_messages_response, remote_model_catalog_poll_delta,
-    remote_model_selection_needs_config, remote_no_change_poll_response, remote_persisted_poll_response,
-    remote_recent_workspaces_response, remote_session_created_response, remote_session_deleted_response,
-    remote_session_info, remote_session_list_response, remote_session_model_updated_response,
-    remote_session_restore_target, remote_snapshot_poll_response, remote_task_cancel_response,
-    remote_workspace_info_response, remote_workspace_updated_response, resolve_remote_agent_type,
-    resolve_remote_cancel_decision, resolve_remote_execution_image_contexts, resolve_remote_file_chunk_range,
-    resolve_remote_workspace_path, should_send_remote_model_catalog, submit_remote_dialog, ActiveTurnSnapshot,
-    ChatImageAttachment, ChatMessage, ChatMessageItem, DeviceIdentity, ImageAttachment, KeyPair, PairingProtocol,
-    PairingState, QrGenerator, QrPayload, RelayMessage, RemoteAssistantWorkspaceFacts, RemoteCancelDecision,
-    RemoteCancelRuntimeHost, RemoteCancelTaskRequest, RemoteChatHistoryRound, RemoteChatHistoryTextItem,
-    RemoteChatHistoryThinkingItem, RemoteChatHistoryToolCall, RemoteChatHistoryToolItem, RemoteChatHistoryTurn,
-    RemoteCommand, RemoteCommandRuntimeHost, RemoteConnectSubmissionSource, RemoteDefaultModelsConfig,
-    RemoteDialogQueuePriority, RemoteDialogResolvedSubmission, RemoteDialogRuntimeHost,
-    RemoteDialogSchedulerOutcomeFact, RemoteDialogSubmissionPolicy, RemoteDialogSubmissionRequest,
-    RemoteDialogSubmitOutcome, RemoteImageContext, RemoteImageContextAdapter, RemoteModelCapabilityFact,
-    RemoteModelCatalog, RemoteModelCatalogFacts, RemoteModelConfig, RemoteModelFacts, RemoteReasoningModeFact,
-    RemoteRecentWorkspaceFacts, RemoteResponse, RemoteSessionMetadata, RemoteSessionStateTracker,
-    RemoteSessionTrackerHost, RemoteSessionTrackerRegistry, RemoteTerminalPrewarmRequest, RemoteToolStatus,
-    RemoteWorkspaceFacts, RemoteWorkspaceFileChunk, RemoteWorkspaceFileContent, RemoteWorkspaceFileInfo,
-    RemoteWorkspaceFileRuntimeHost, RemoteWorkspaceKind, RemoteWorkspaceUpdate, TrackerEvent,
-    REMOTE_FILE_MAX_CHUNK_BYTES, REMOTE_FILE_MAX_READ_BYTES,
-};
 pub use serde_json::json;
 pub use std::path::PathBuf;
 pub use std::sync::{Arc, Mutex};
 
-#[derive(Debug, Clone, PartialEq)]
-pub struct TestImageContext {
-    pub id: String,
-    pub image_path: Option<String>,
-    pub data_url: Option<String>,
-    pub mime_type: String,
-    pub metadata: Option<serde_json::Value>,
-}
-
-impl RemoteImageContextAdapter for TestImageContext {
-    fn from_remote_image_context(context: RemoteImageContext) -> Self {
-        Self {
-            id: context.id,
-            image_path: context.image_path,
-            data_url: context.data_url,
-            mime_type: context.mime_type,
-            metadata: context.metadata,
-        }
-    }
-}
-
-#[test]
-fn remote_connect_image_context_adapter_owns_portable_conversion_shape() {
-    let context = RemoteImageContext {
-        id: "ctx-1".to_string(),
-        image_path: Some("D:/workspace/project/screenshot.png".to_string()),
-        data_url: Some("data:image/png;base64,abc".to_string()),
-        mime_type: "image/png".to_string(),
-        metadata: Some(serde_json::json!({ "source": "remote" })),
-    };
-
-    let adapted = TestImageContext::from_remote_image_context(context);
-
-    assert_eq!(adapted.id, "ctx-1");
-    assert_eq!(
-        adapted.image_path.as_deref(),
-        Some("D:/workspace/project/screenshot.png")
-    );
-    assert_eq!(adapted.data_url.as_deref(), Some("data:image/png;base64,abc"));
-    assert_eq!(adapted.mime_type, "image/png");
-    assert_eq!(adapted.metadata.as_ref().unwrap()["source"], "remote");
-}
-
-pub fn remote_history_contract_turn(is_in_progress: bool) -> RemoteChatHistoryTurn {
-    RemoteChatHistoryTurn {
-        turn_id: "turn-1".to_string(),
-        user_message_id: "user-1".to_string(),
-        user_display_content: "original question".to_string(),
-        user_timestamp_ms: 1_000,
-        user_images: vec![ChatImageAttachment {
-            name: "screenshot.png".to_string(),
-            data_url: "data:image/png;base64,abcd".to_string(),
-        }],
-        is_in_progress,
-        start_time_ms: 1_000,
-        rounds: vec![RemoteChatHistoryRound {
-            start_time_ms: 1_100,
-            end_time_ms: Some(1_200),
-            text_items: vec![
-                RemoteChatHistoryTextItem {
-                    content: "hidden text".to_string(),
-                    order_index: Some(1),
-                    is_subagent: true,
-                },
-                RemoteChatHistoryTextItem {
-                    content: "visible text".to_string(),
-                    order_index: Some(1),
-                    is_subagent: false,
-                },
-            ],
-            thinking_items: vec![RemoteChatHistoryThinkingItem {
-                content: "visible thought".to_string(),
-                order_index: Some(0),
-                is_subagent: false,
-            }],
-            tool_items: vec![RemoteChatHistoryToolItem {
-                id: "tool-1".to_string(),
-                name: "AskUserQuestion".to_string(),
-                call: RemoteChatHistoryToolCall {
-                    id: "call-1".to_string(),
-                    input: serde_json::json!({ "question": "confirm?" }),
-                },
-                has_result: false,
-                status: Some("running".to_string()),
-                duration_ms: Some(25),
-                start_ms: 1_130,
-                order_index: Some(2),
-                is_subagent: false,
-            }],
-        }],
-    }
-}
-
-pub struct RecordingDialogHost {
-    pub session_exists: bool,
-    pub binding_workspace: Option<String>,
-    pub generated_turn_id: String,
-    pub restore_error: bool,
-    pub submit_outcome: RemoteDialogSubmitOutcome,
-    pub events: Mutex<Vec<String>>,
-    pub submitted: Mutex<Option<RemoteDialogResolvedSubmission<String>>>,
-}
-
-impl RecordingDialogHost {
-    pub fn new(session_exists: bool, binding_workspace: Option<&str>) -> Self {
-        Self {
-            session_exists,
-            binding_workspace: binding_workspace.map(ToOwned::to_owned),
-            generated_turn_id: "turn-generated".to_string(),
-            restore_error: false,
-            submit_outcome: RemoteDialogSubmitOutcome::Started {
-                session_id: "session-1".to_string(),
-                turn_id: "turn-generated".to_string(),
-            },
-            events: Mutex::new(Vec::new()),
-            submitted: Mutex::new(None),
-        }
-    }
-
-    pub fn with_restore_error(mut self) -> Self {
-        self.restore_error = true;
-        self
-    }
-
-    pub fn with_submit_outcome(mut self, submit_outcome: RemoteDialogSubmitOutcome) -> Self {
-        self.submit_outcome = submit_outcome;
-        self
-    }
-
-    pub fn events(&self) -> Vec<String> {
-        self.events.lock().unwrap().clone()
-    }
-
-    pub fn submitted(&self) -> RemoteDialogResolvedSubmission<String> {
-        self.submitted.lock().unwrap().clone().expect("dialog submitted")
-    }
-}
-
-#[async_trait::async_trait]
-impl RemoteDialogRuntimeHost for RecordingDialogHost {
-    type ImageContext = String;
-
-    fn ensure_tracker(&self, session_id: &str) {
-        self.events.lock().unwrap().push(format!("ensure_tracker:{session_id}"));
-    }
-
-    async fn resolve_binding_workspace(&self, session_id: &str) -> Option<String> {
-        self.events
-            .lock()
-            .unwrap()
-            .push(format!("resolve_workspace:{session_id}"));
-        self.binding_workspace.clone()
-    }
-
-    async fn remote_session_exists(&self, session_id: &str) -> Result<bool, String> {
-        self.events.lock().unwrap().push(format!("session_exists:{session_id}"));
-        Ok(self.session_exists)
-    }
-
-    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String> {
-        self.events
-            .lock()
-            .unwrap()
-            .push(format!("restore:{session_id}:{workspace_path}"));
-        if self.restore_error {
-            Err("restore failed".to_string())
-        } else {
-            Ok(())
-        }
-    }
-
-    fn prewarm_remote_terminal(&self, request: RemoteTerminalPrewarmRequest) {
-        self.events.lock().unwrap().push(format!(
-            "prewarm:{}:{}",
-            request.session_id,
-            request.binding_workspace.as_deref().unwrap_or("<none>")
-        ));
-    }
-
-    fn generate_turn_id(&self) -> String {
-        self.events.lock().unwrap().push("generate_turn".to_string());
-        self.generated_turn_id.clone()
-    }
-
-    async fn submit_dialog(
-        &self,
-        submission: RemoteDialogResolvedSubmission<Self::ImageContext>,
-    ) -> Result<RemoteDialogSubmitOutcome, String> {
-        self.events
-            .lock()
-            .unwrap()
-            .push(format!("submit:{}", submission.session_id));
-        *self.submitted.lock().unwrap() = Some(submission);
-        Ok(self.submit_outcome.clone())
-    }
-}
-
-pub struct RecordingCancelHost {
-    pub initial_state: Mutex<Option<RemoteControlStateSnapshot>>,
-    pub restored_state: Mutex<Option<RemoteControlStateSnapshot>>,
-    pub state_reads: Mutex<usize>,
-    pub restore_workspace: Option<String>,
-    pub restore_error: bool,
-    pub cancel_error: Option<String>,
-    pub events: Mutex<Vec<String>>,
-}
-
-impl RecordingCancelHost {
-    pub fn new(
-        initial_state: Option<RemoteControlStateSnapshot>,
-        restored_state: Option<RemoteControlStateSnapshot>,
-        restore_workspace: Option<&str>,
-    ) -> Self {
-        Self {
-            initial_state: Mutex::new(initial_state),
-            restored_state: Mutex::new(restored_state),
-            state_reads: Mutex::new(0),
-            restore_workspace: restore_workspace.map(ToOwned::to_owned),
-            restore_error: false,
-            cancel_error: None,
-            events: Mutex::new(Vec::new()),
-        }
-    }
-
-    pub fn with_restore_error(mut self) -> Self {
-        self.restore_error = true;
-        self
-    }
-
-    pub fn events(&self) -> Vec<String> {
-        self.events.lock().unwrap().clone()
-    }
-}
-
-pub fn remote_state(
-    session_id: &str,
-    state: RemoteControlSessionState,
-    active_turn_id: Option<&str>,
-) -> RemoteControlStateSnapshot {
-    RemoteControlStateSnapshot {
-        session_id: session_id.to_string(),
-        state,
-        active_turn_id: active_turn_id.map(ToOwned::to_owned),
-        queue_depth: 0,
-        metadata: serde_json::Map::new(),
-    }
-}
-
-#[async_trait::async_trait]
-impl RemoteCancelRuntimeHost for RecordingCancelHost {
-    async fn resolve_restore_workspace(&self, session_id: &str) -> Option<String> {
-        self.events
-            .lock()
-            .unwrap()
-            .push(format!("resolve_workspace:{session_id}"));
-        self.restore_workspace.clone()
-    }
-
-    async fn remote_control_state(&self, session_id: &str) -> Result<Option<RemoteControlStateSnapshot>, String> {
-        self.events.lock().unwrap().push(format!("read_state:{session_id}"));
-        let mut reads = self.state_reads.lock().unwrap();
-        let read_index = *reads;
-        *reads += 1;
-        drop(reads);
-
-        if read_index == 0 {
-            return Ok(self.initial_state.lock().unwrap().clone());
-        }
-        Ok(self.restored_state.lock().unwrap().clone())
-    }
-
-    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String> {
-        self.events
-            .lock()
-            .unwrap()
-            .push(format!("restore:{session_id}:{workspace_path}"));
-        if self.restore_error {
-            Err("restore failed".to_string())
-        } else {
-            Ok(())
-        }
-    }
-
-    async fn cancel_remote_turn(&self, session_id: &str, turn_id: &str) -> Result<(), String> {
-        self.events
-            .lock()
-            .unwrap()
-            .push(format!("cancel:{session_id}:{turn_id}"));
-        if let Some(error) = &self.cancel_error {
-            Err(error.clone())
-        } else {
-            Ok(())
-        }
-    }
-}
-
-#[derive(Default)]
-pub struct RecordingCommandHost {
-    pub events: Mutex<Vec<String>>,
-    pub submitted_dialog: Mutex<Option<RemoteDialogSubmissionRequest<String>>>,
-    pub cancel_request: Mutex<Option<RemoteCancelTaskRequest>>,
-    pub explicit_context_ids: Mutex<Vec<String>>,
-    pub legacy_image_names: Mutex<Vec<String>>,
-}
-
-impl RecordingCommandHost {
-    pub fn events(&self) -> Vec<String> {
-        self.events.lock().unwrap().clone()
-    }
-
-    pub fn submitted_dialog(&self) -> RemoteDialogSubmissionRequest<String> {
-        self.submitted_dialog.lock().unwrap().clone().expect("dialog submitted")
-    }
-
-    pub fn cancel_request(&self) -> RemoteCancelTaskRequest {
-        self.cancel_request.lock().unwrap().clone().expect("cancel requested")
-    }
-}
-
-#[async_trait::async_trait]
-impl RemoteCommandRuntimeHost for RecordingCommandHost {
-    type ImageContext = String;
-
-    async fn handle_workspace_command(&self, _command: &RemoteCommand) -> RemoteResponse {
-        self.events.lock().unwrap().push("workspace".to_string());
-        RemoteResponse::WorkspaceInfo {
-            has_workspace: false,
-            path: None,
-            project_name: None,
-            git_branch: None,
-            workspace_kind: None,
-            assistant_id: None,
-        }
-    }
-
-    async fn handle_session_command(&self, _command: &RemoteCommand) -> RemoteResponse {
-        self.events.lock().unwrap().push("session".to_string());
-        RemoteResponse::SessionCreated {
-            session_id: "session-created".to_string(),
-        }
-    }
-
-    async fn handle_poll_command(&self, _command: &RemoteCommand) -> RemoteResponse {
-        self.events.lock().unwrap().push("poll".to_string());
-        RemoteResponse::SessionPoll {
-            version: 0,
-            changed: false,
-            session_state: None,
-            title: None,
-            new_messages: None,
-            total_msg_count: None,
-            active_turn: None,
-            model_catalog: Box::new(None),
-        }
-    }
-
-    async fn handle_workspace_file_command(&self, _command: &RemoteCommand) -> RemoteResponse {
-        self.events.lock().unwrap().push("file".to_string());
-        RemoteResponse::FileInfo {
-            name: "file.txt".to_string(),
-            size: 1,
-            mime_type: "text/plain".to_string(),
-        }
-    }
-
-    async fn handle_interaction_command(&self, _command: &RemoteCommand) -> RemoteResponse {
-        self.events.lock().unwrap().push("interaction".to_string());
-        RemoteResponse::InteractionAccepted {
-            action: "confirm_tool".to_string(),
-            target_id: "tool-1".to_string(),
-        }
-    }
-
-    async fn submit_dialog(
-        &self,
-        request: RemoteDialogSubmissionRequest<Self::ImageContext>,
-    ) -> Result<RemoteDialogSubmitOutcome, String> {
-        self.events.lock().unwrap().push("submit".to_string());
-        *self.submitted_dialog.lock().unwrap() = Some(request.clone());
-        Ok(RemoteDialogSubmitOutcome::Started {
-            session_id: request.session_id,
-            turn_id: "turn-command".to_string(),
-        })
-    }
-
-    async fn cancel_task(&self, request: RemoteCancelTaskRequest) -> Result<(), String> {
-        self.events.lock().unwrap().push("cancel".to_string());
-        *self.cancel_request.lock().unwrap() = Some(request);
-        Ok(())
-    }
-
-    fn legacy_image_contexts(&self, images: Option<&[ImageAttachment]>) -> Vec<Self::ImageContext> {
-        let names = images
-            .unwrap_or_default()
-            .iter()
-            .map(|image| image.name.clone())
-            .collect::<Vec<_>>();
-        *self.legacy_image_names.lock().unwrap() = names.clone();
-        names.into_iter().map(|name| format!("legacy:{name}")).collect()
-    }
-
-    fn explicit_image_contexts(&self, contexts: Vec<RemoteImageContext>) -> Vec<Self::ImageContext> {
-        let ids = contexts.into_iter().map(|context| context.id).collect::<Vec<_>>();
-        *self.explicit_context_ids.lock().unwrap() = ids.clone();
-        ids.into_iter().map(|id| format!("explicit:{id}")).collect()
-    }
-}
-
-pub fn make_temp_remote_workspace() -> (PathBuf, PathBuf, PathBuf) {
-    let base = std::env::temp_dir().join(format!("northhing-remote-connect-contract-{}", uuid::Uuid::new_v4()));
-    let workspace = base.join("workspace");
-    let artifacts = workspace.join("artifacts");
-    std::fs::create_dir_all(&artifacts).expect("create remote workspace");
-    let report = artifacts.join("report.md");
-    std::fs::write(&report, b"hello remote file").expect("write remote file");
-    (base, workspace, report)
-}
-
-#[derive(Default)]
-pub struct RecordingFileHost {
-    pub workspace_root: PathBuf,
-    pub seen_sessions: Mutex<Vec<Option<String>>>,
-}
-
-#[async_trait::async_trait]
-impl RemoteWorkspaceFileRuntimeHost for RecordingFileHost {
-    async fn resolve_remote_file_workspace_root(&self, session_id: Option<&str>) -> Option<PathBuf> {
-        self.seen_sessions
-            .lock()
-            .unwrap()
-            .push(session_id.map(ToOwned::to_owned));
-        Some(self.workspace_root.clone())
-    }
-}
-
-pub fn sample_remote_model_catalog(version: u64) -> RemoteModelCatalog {
-    RemoteModelCatalog {
-        version,
-        models: vec![RemoteModelConfig {
-            id: "model-1".to_string(),
-            name: "Model One".to_string(),
-            provider: "openai".to_string(),
-            base_url: "https://api.example.com".to_string(),
-            model_name: "gpt-test".to_string(),
-            context_window: Some(128_000),
-            enabled: true,
-            capabilities: vec!["text_chat".to_string()],
-            enable_thinking_process: false,
-            reasoning_mode: Some("default".to_string()),
-            reasoning_effort: None,
-            thinking_budget_tokens: None,
-        }],
-        default_models: RemoteDefaultModelsConfig {
-            primary: Some("model-1".to_string()),
-            ..RemoteDefaultModelsConfig::default()
-        },
-        session_model_id: Some("model-1".to_string()),
-    }
-}
-
-#[derive(Default)]
-pub struct RecordingTrackerHost {
-    pub subscribed: Mutex<Vec<String>>,
-    pub unsubscribed: Mutex<Vec<String>>,
-    pub active_turn_id: Mutex<Option<String>>,
-}
-
-impl RecordingTrackerHost {
-    pub fn with_active_turn(turn_id: impl Into<String>) -> Self {
-        Self {
-            active_turn_id: Mutex::new(Some(turn_id.into())),
-            ..Self::default()
-        }
-    }
-}
-
-impl RemoteSessionTrackerHost for RecordingTrackerHost {
-    fn subscribe_tracker(&self, session_id: &str, _tracker: Arc<RemoteSessionStateTracker>) {
-        self.subscribed.lock().unwrap().push(session_id.to_string());
-    }
-
-    fn unsubscribe_tracker(&self, session_id: &str) {
-        self.unsubscribed.lock().unwrap().push(session_id.to_string());
-    }
-
-    fn active_turn_id(&self, _session_id: &str) -> Option<String> {
-        self.active_turn_id.lock().unwrap().clone()
-    }
-}
-
 pub use northhing_services_integrations::mcp::auth::{MCPRemoteOAuthSessionSnapshot, MCPRemoteOAuthStatus};
 pub use northhing_services_integrations::mcp::config::ConfigLocation;
 pub use northhing_services_integrations::mcp::config::{
     config_to_cursor_format, format_mcp_json_config_value, get_mcp_remote_authorization_source,
     get_mcp_remote_authorization_value, has_mcp_remote_authorization, has_mcp_remote_oauth, has_mcp_remote_xaa,
     merge_mcp_server_config_sources, normalize_mcp_authorization_value, parse_cursor_format,
     remove_mcp_authorization_keys, validate_mcp_json_config, MCPConfigService, MCPConfigStore,
 };
 pub use northhing_services_integrations::mcp::protocol::{
     create_initialize_request, create_ping_request, create_tools_call_request, create_tools_list_request,
