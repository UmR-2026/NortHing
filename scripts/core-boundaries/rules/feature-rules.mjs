// Boundary rules for feature assembly and optional dependency ownership.

export const optionalDependencyFeatureOwnerRules = [
  {
    crateName: 'core',
    reason:
      'northhing-core product/runtime optional dependencies must stay owned by explicit feature gates',
    dependencies: [
      { depName: 'aes', ownerFeatures: ['service-integrations'] },
      { depName: 'aes-gcm', ownerFeatures: ['service-integrations'] },
      { depName: 'axum', ownerFeatures: ['service-integrations'] },
      { depName: 'northhing-ai-adapters', ownerFeatures: ['ai-adapter-runtime'] },
      { depName: 'northhing-product-capabilities', ownerFeatures: ['product-capabilities'] },
      { depName: 'northhing-product-domains', ownerFeatures: ['product-domains'] },
      { depName: 'chrono-tz', ownerFeatures: ['product-full'] },
      { depName: 'cron', ownerFeatures: ['product-full'] },
      { depName: 'dashmap', ownerFeatures: ['product-full'] },
      { depName: 'eventsource-stream', ownerFeatures: ['product-full'] },
      { depName: 'filetime', ownerFeatures: ['product-full'] },
      { depName: 'flate2', ownerFeatures: ['product-full'] },
      { depName: 'fs2', ownerFeatures: ['product-full'] },
      { depName: 'git2', ownerFeatures: ['service-integrations'] },
      { depName: 'glob', ownerFeatures: ['product-full'] },
      { depName: 'globset', ownerFeatures: ['product-full'] },
      { depName: 'image', ownerFeatures: ['service-integrations'] },
      { depName: 'include_dir', ownerFeatures: ['product-full'] },
      { depName: 'indexmap', ownerFeatures: ['product-full'] },
      { depName: 'local-ip-address', ownerFeatures: ['service-integrations'] },
      { depName: 'md5', ownerFeatures: ['product-full', 'service-integrations'] },
      { depName: 'rand', ownerFeatures: ['service-integrations'] },
      { depName: 'reqwest', ownerFeatures: ['ai-adapter-runtime', 'service-integrations'] },
      { depName: 'rmcp', ownerFeatures: ['service-integrations'] },
      { depName: 'russh', ownerFeatures: ['ssh-remote'] },
      { depName: 'similar', ownerFeatures: ['product-full'] },
      { depName: 'sse-stream', ownerFeatures: ['service-integrations'] },
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
      { depName: 'aes-gcm', ownerFeatures: ['mcp', 'remote-ssh-concrete'] },
      { depName: 'anyhow', ownerFeatures: ['mcp', 'remote-ssh-concrete'] },
      {
        depName: 'base64',
        ownerFeatures: ['mcp', 'remote-ssh-concrete'],
      },
      { depName: 'northhing-product-domains', ownerFeatures: ['function-agents'] },
      { depName: 'northhing-runtime-ports', ownerFeatures: ['deep-research'] },
      {
        depName: 'northhing-services-core',
        ownerFeatures: ['git', 'mcp', 'workspace-search', 'remote-ssh-concrete'],
      },
      { depName: 'chrono', ownerFeatures: ['git', 'remote-ssh-concrete'] },
      { depName: 'dirs', ownerFeatures: ['remote-ssh-concrete'] },
      { depName: 'dunce', ownerFeatures: ['remote-ssh', 'workspace-search'] },
      { depName: 'futures', ownerFeatures: ['mcp'] },
      { depName: 'git2', ownerFeatures: ['git'] },
      { depName: 'notify', ownerFeatures: ['file-watch'] },
      { depName: 'rand', ownerFeatures: ['mcp', 'remote-ssh-concrete'] },
      { depName: 'reqwest', ownerFeatures: ['mcp'] },
      { depName: 'rmcp', ownerFeatures: ['mcp'] },
      { depName: 'russh', ownerFeatures: ['remote-ssh-concrete'] },
      { depName: 'russh-sftp', ownerFeatures: ['remote-ssh-concrete'] },
      { depName: 'sha2', ownerFeatures: ['remote-ssh'] },
      { depName: 'shellexpand', ownerFeatures: ['remote-ssh-concrete'] },
      { depName: 'sse-stream', ownerFeatures: ['mcp'] },
      { depName: 'ssh_config', ownerFeatures: ['remote-ssh-concrete', 'ssh_config'] },
      { depName: 'terminal-core', ownerFeatures: ['remote-ssh-concrete'] },
      { depName: 'thiserror', ownerFeatures: ['git', 'remote-ssh-concrete', 'workspace-search'] },
      { depName: 'tokio-util', ownerFeatures: ['remote-ssh'] },
      { depName: 'uuid', ownerFeatures: ['remote-ssh-concrete'] },
      { depName: 'which', ownerFeatures: ['workspace-search'] },
    ],
  },
  {
    crateName: 'product-domains',
    reason:
      'product-domains optional runtime dependencies must stay owned by explicit product-domain features',
    dependencies: [],
  },
];

export const productCoreFeatureAssemblyRules = [
  {
    manifestPath: 'src/apps/desktop/Cargo.toml',
    dependencyName: 'northhing-core',
    requiredFeatures: ['product-full'],
    reason: 'desktop must explicitly assemble the full northhing-core product runtime',
  },
  {
    manifestPath: 'src/apps/cli/Cargo.toml',
    dependencyName: 'northhing-core',
    requiredFeatures: ['product-full'],
    reason: 'CLI must explicitly assemble the full northhing-core product runtime',
  },
  {
    manifestPath: 'src/crates/interfaces/acp/Cargo.toml',
    dependencyName: 'northhing-core',
    requiredFeatures: ['product-full'],
    reason: 'ACP must explicitly assemble the full northhing-core product runtime',
  },
];

export const productCoreFeatureAssemblyScanRoots = [
  'src/apps',
  'src/crates/interfaces/acp',
];

export const coreProductFullFeatureAssemblyRule = {
  manifestPath: 'src/crates/assembly/core/Cargo.toml',
  featureName: 'product-full',
  requiredFeatureRefs: [
    'ssh-remote',
    'product-capabilities',
    'product-domains',
    'service-integrations',
  ],
  reason: 'northhing-core product-full must explicitly assemble current owner feature groups',
};

export const ownerCrateFeatureAssemblyRules = [
  {
    manifestPath: 'src/crates/services/services-integrations/Cargo.toml',
    reason: 'services-integrations must keep integration feature groups explicit and default-light',
    requiredProductFullFeatures: [
      'announcement',
      'deep-research',
      'file-watch',
      'function-agents',
      'git',
      'mcp',
      'remote-ssh',
      'remote-ssh-concrete',
      'workspace-search',
    ],
  },
  {
    manifestPath: 'src/crates/contracts/product-domains/Cargo.toml',
    reason: 'product-domains must keep product domain feature groups explicit and default-light',
    requiredProductFullFeatures: ['function-agents'],
  },
];
