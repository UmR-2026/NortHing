BASE: eb43877 (working-tree diff, task not yet committed)

## numstat (raw)
0	74	scripts/generate-i18n-contract.mjs
110	329	scripts/i18n-audit.mjs
0	89	scripts/i18n-contract.test.mjs
0	2	scripts/i18n-governance-baseline.json
0	4	scripts/i18n-hardcoded-baseline.json
0	10	src/shared/i18n/contract/locales.json

## numstat (-w whitespace-insensitive) — i18n-audit.mjs purity proof
0	219	scripts/i18n-audit.mjs

## mojibake consistency: node --check output (base line 503 -> current 481, same error)
E:\agent-project\northing\scripts\i18n-audit.mjs:481
  '猫驴?,
  ^^^^^


## git diff -U10
diff --git a/scripts/generate-i18n-contract.mjs b/scripts/generate-i18n-contract.mjs
index 7536f85..1dfce91 100644
--- a/scripts/generate-i18n-contract.mjs
+++ b/scripts/generate-i18n-contract.mjs
@@ -4,24 +4,20 @@ import path from 'node:path';
 
 const root = process.cwd();
 const checkOnly = process.argv.includes('--check');
 const contractPath = path.join(root, 'src', 'shared', 'i18n', 'contract', 'locales.json');
 
 const outputs = [
   {
     path: path.join(root, 'src', 'web-ui', 'src', 'infrastructure', 'i18n', 'presets', 'generatedLocaleContract.ts'),
     generate: generateWebLocaleContract,
   },
-  {
-    path: path.join(root, 'src', 'mobile-web', 'src', 'i18n', 'generatedLocaleContract.ts'),
-    generate: generateMobileLocaleContract,
-  },
   {
     path: path.join(root, 'northhing-Installer', 'src', 'i18n', 'generatedLocaleContract.ts'),
     generate: generateInstallerLocaleContract,
   },
   {
     path: path.join(root, 'src', 'crates', 'assembly', 'core', 'src', 'service', 'i18n', 'generated_locale_contract.rs'),
     generate: generateCoreRustLocaleContract,
   },
   {
     path: path.join(root, 'northhing-Installer', 'src-tauri', 'src', 'installer', 'generated_locale_contract.rs'),
@@ -280,90 +276,20 @@ export function getLocaleFallbackChain(localeId: string, includeSelf = false): L
 
 export const CONTRACT_LOCALE_METADATA_BY_ID = {
 ${contract.locales.map((locale) => {
   const webLocale = localeMap.get(locale.id);
   return `  ${jsonString(locale.id)}: ${jsonString(webLocale.englishName)}`;
 }).join(',\n')}
 } as const satisfies Record<LocaleId, string>;
 `;
 }
 
-function generateMobileLocaleContract(contract, sharedTermsByLocale) {
-  const locales = orderedLocales(contract, 'mobile-web');
-  const defaultLanguage = contract.surfaceDefaults['mobile-web'];
-  const unknownFallbacks = contract.unknownLocaleFallbackChain;
-  const sharedTerms = sharedTermsForLocales(sharedTermsByLocale, locales);
-
-  return `${generatedHeader('ts')}export const MOBILE_LOCALES = [
-${locales.map((locale) => `  {
-    id: ${jsonString(locale.id)},
-    shortName: ${jsonString(locale.shortName)},
-    aliases: ${tsArray(locale.aliases)},
-    contentFallbacks: ${tsArray(locale.contentFallbacks)},
-  }`).join(',\n')}
-] as const;
-const UNKNOWN_LANGUAGE_FALLBACK_CHAIN = ${tsArray(unknownFallbacks)} as const satisfies readonly MobileLanguage[];
-
-const mobileLocaleAliasesByPriority = MOBILE_LOCALES
-  .flatMap(locale => locale.aliases.map(alias => ({ locale, alias: alias.toLowerCase() })))
-  .sort((a, b) => b.alias.length - a.alias.length);
-
-export type MobileLanguage = (typeof MOBILE_LOCALES)[number]['id'];
-
-export const DEFAULT_LANGUAGE = ${jsonString(defaultLanguage)} satisfies MobileLanguage;
-
-export type SharedI18nTerms = {
-  readonly [key: string]: string | SharedI18nTerms;
-};
-
-export const SHARED_TERMS_BY_LOCALE = ${tsObject(sharedTerms)} as const satisfies Record<MobileLanguage, SharedI18nTerms>;
-
-export function isMobileLanguage(value: string | null | undefined): value is MobileLanguage {
-  return MOBILE_LOCALES.some(locale => locale.id === value);
-}
-
-export function resolveMobileLanguage(value: string | null | undefined): MobileLanguage | null {
-  const normalized = value?.trim().toLowerCase();
-  if (!normalized) return null;
-
-  const exact = MOBILE_LOCALES.find(locale => locale.id.toLowerCase() === normalized);
-  if (exact) return exact.id;
-
-  return mobileLocaleAliasesByPriority
-    .find(({ alias }) => normalized === alias || normalized.startsWith(\`\${alias}-\`))
-    ?.locale.id ?? null;
-}
-
-export function getNextMobileLanguage(language: MobileLanguage): MobileLanguage {
-  const currentIndex = MOBILE_LOCALES.findIndex(locale => locale.id === language);
-  return MOBILE_LOCALES[(currentIndex + 1) % MOBILE_LOCALES.length].id;
-}
-
-export function getMobileLanguageShortName(language: MobileLanguage): string {
-  return MOBILE_LOCALES.find(locale => locale.id === language)?.shortName ?? language;
-}
-
-export function getMobileFallbackChain(language: string | null | undefined, includeSelf = false): MobileLanguage[] {
-  const resolved = resolveMobileLanguage(language);
-  const locale = resolved ? MOBILE_LOCALES.find(item => item.id === resolved) : null;
-  const chain: MobileLanguage[] = locale
-    ? [
-      ...(includeSelf ? [locale.id] : []),
-      ...locale.contentFallbacks,
-    ]
-    : [...UNKNOWN_LANGUAGE_FALLBACK_CHAIN];
-
-  return Array.from(new Set(chain));
-}
-`;
-}
-
 function generateInstallerLocaleContract(contract, sharedTermsByLocale) {
   const locales = orderedLocales(contract, 'installer');
   const defaultAppLanguage = contract.surfaceDefaults.installer;
   const defaultLocale = getLocaleMap(contract).get(defaultAppLanguage);
   const sharedTerms = sharedTermsForLocales(sharedTermsByLocale, locales);
 
   return `${generatedHeader('ts')}export interface InstallerLanguageDefinition {
   uiCode: string;
   appCode: string;
   label: string;
diff --git a/scripts/i18n-audit.mjs b/scripts/i18n-audit.mjs
index 50818e1..1fda00e 100644
--- a/scripts/i18n-audit.mjs
+++ b/scripts/i18n-audit.mjs
@@ -17,24 +17,22 @@ const webLocalesDir = path.join(root, 'src', 'web-ui', 'src', 'locales');
 const namespaceRegistryPath = path.join(
   root,
   'src',
   'web-ui',
   'src',
   'infrastructure',
   'i18n',
   'presets',
   'namespaceRegistry.ts',
 );
-const webSourceDir = path.join(root, 'src', 'web-ui', 'src');
-const mobileWebSourceDir = path.join(root, 'src', 'mobile-web', 'src');
-const mobileWebMessagesPath = path.join(mobileWebSourceDir, 'i18n', 'messages.ts');
-const installerSourceDir = path.join(root, 'northhing-Installer', 'src');
+const webSourceDir = path.join(root, 'src', 'web-ui', 'src');
+const installerSourceDir = path.join(root, 'northhing-Installer', 'src');
 const installerLocalesDir = path.join(installerSourceDir, 'i18n', 'locales');
 const coreLocalesDir = path.join(root, 'src', 'crates', 'assembly', 'core', 'locales');
 const supportedLocales = fs
   .readdirSync(webLocalesDir, { withFileTypes: true })
   .filter((entry) => entry.isDirectory())
   .map((entry) => entry.name)
   .sort();
 const baselineLocale = supportedLocales.includes('en-US') ? 'en-US' : supportedLocales[0];
 const localeContract = readJsonFile(contractPath);
 
@@ -214,31 +212,26 @@ function flattenStringEntries(value, prefix = '') {
 }
 
 function sortedUnique(values) {
   return Array.from(new Set(values)).sort();
 }
 
 function isPlainObject(value) {
   return value != null && typeof value === 'object' && !Array.isArray(value);
 }
 
-function extractI18nextPlaceholders(value) {
-  const matches = String(value).matchAll(/\{\{\s*-?\s*([A-Za-z_][\w]*)\s*\}\}/g);
-  return sortedUnique(Array.from(matches, (match) => match[1]));
-}
-
-function extractMobilePlaceholders(value) {
-  const matches = String(value).matchAll(/\{\s*([A-Za-z_][\w]*)\s*\}/g);
-  return sortedUnique(Array.from(matches, (match) => match[1]));
-}
-
-function extractFluentPlaceholders(value) {
+function extractI18nextPlaceholders(value) {
+  const matches = String(value).matchAll(/\{\{\s*-?\s*([A-Za-z_][\w]*)\s*\}\}/g);
+  return sortedUnique(Array.from(matches, (match) => match[1]));
+}
+
+function extractFluentPlaceholders(value) {
   const matches = String(value).matchAll(/\$\s*([A-Za-z_][\w-]*)/g);
   return sortedUnique(Array.from(matches, (match) => match[1]));
 }
 
 function sameSet(left, right) {
   if (left.length !== right.length) return false;
   return left.every((item, index) => item === right[index]);
 }
 
 function hasHanText(value) {
@@ -487,127 +480,29 @@ function readInstallerJsonEntries(uiLocale) {
   }
 }
 
 function propertyNameToString(ts, name) {
   if (ts.isIdentifier(name) || ts.isStringLiteral(name) || ts.isNumericLiteral(name)) {
     return name.text;
   }
   return null;
 }
 
-function unwrapTsExpression(ts, expression) {
-  let current = expression;
-  while (current && (ts.isAsExpression(current) || ts.isSatisfiesExpression(current))) {
-    current = current.expression;
-  }
-  return current;
-}
-
-function flattenTsObjectKeys(ts, objectLiteral, prefix = '') {
-  const keys = [];
-  for (const property of objectLiteral.properties) {
-    if (!ts.isPropertyAssignment(property)) continue;
-
-    const key = propertyNameToString(ts, property.name);
-    if (!key) continue;
-    if (!prefix && key === 'shared') continue;
-
-    const nextPrefix = prefix ? `${prefix}.${key}` : key;
-    const initializer = unwrapTsExpression(ts, property.initializer);
-
-    if (ts.isObjectLiteralExpression(initializer)) {
-      keys.push(...flattenTsObjectKeys(ts, initializer, nextPrefix));
-    } else {
-      keys.push(nextPrefix);
-    }
-  }
-  return keys.sort();
-}
-
-function flattenTsObjectEntries(ts, objectLiteral, prefix = '') {
-  const entries = [];
-  for (const property of objectLiteral.properties) {
-    if (!ts.isPropertyAssignment(property)) continue;
-
-    const key = propertyNameToString(ts, property.name);
-    if (!key) continue;
-    if (!prefix && key === 'shared') continue;
-
-    const nextPrefix = prefix ? `${prefix}.${key}` : key;
-    const initializer = unwrapTsExpression(ts, property.initializer);
-
-    if (ts.isObjectLiteralExpression(initializer)) {
-      entries.push(...flattenTsObjectEntries(ts, initializer, nextPrefix));
-    } else if (
-      ts.isStringLiteral(initializer) ||
-      ts.isNoSubstitutionTemplateLiteral(initializer)
-    ) {
-      entries.push([nextPrefix, initializer.text]);
-    } else {
-      entries.push([nextPrefix, '']);
-    }
-  }
-  return entries.sort(([left], [right]) => left.localeCompare(right));
-}
-
-function readMobileMessagesByLocale() {
-  const ts = auditTypeScript;
-  if (!ts) {
-    return new Map();
-  }
-
-  const source = fs.readFileSync(mobileWebMessagesPath, 'utf8');
-  const sourceFile = ts.createSourceFile(mobileWebMessagesPath, source, ts.ScriptTarget.Latest, true);
-  const output = new Map();
-
-  function visit(node) {
-    if (
-      ts.isVariableDeclaration(node) &&
-      ts.isIdentifier(node.name) &&
-      node.name.text === 'messages'
-    ) {
-      const initializer = unwrapTsExpression(ts, node.initializer);
-      if (!initializer || !ts.isObjectLiteralExpression(initializer)) {
-        reportError('mobile-web messages export is not an object literal');
-        return;
-      }
-
-      for (const property of initializer.properties) {
-        if (!ts.isPropertyAssignment(property)) continue;
-
-        const locale = propertyNameToString(ts, property.name);
-        if (!locale) continue;
-
-        const value = unwrapTsExpression(ts, property.initializer);
-        if (!ts.isObjectLiteralExpression(value)) {
-          reportError(`mobile-web messages.${locale} is not an object literal`);
-          continue;
-        }
-
-        output.set(locale, new Map(flattenTsObjectEntries(ts, value)));
-      }
-    }
-    ts.forEachChild(node, visit);
-  }
-
-  visit(sourceFile);
-  return output;
-}
-
-function readMobileMessageKeysByLocale() {
-  return new Map(
-    Array.from(readMobileMessagesByLocale().entries())
-      .map(([locale, entries]) => [locale, Array.from(entries.keys()).sort()]),
-  );
-}
-
-function diffSets(left, right) {
+function unwrapTsExpression(ts, expression) {
+  let current = expression;
+  while (current && (ts.isAsExpression(current) || ts.isSatisfiesExpression(current))) {
+    current = current.expression;
+  }
+  return current;
+}
+
+function diffSets(left, right) {
   const rightSet = new Set(right);
   return left.filter((item) => !rightSet.has(item));
 }
 
 function auditNamespaceCoverage() {
   const registryLocales = readRegistryLocales();
   for (const locale of supportedLocales.filter((item) => !registryLocales.includes(item))) {
     reportError(`${locale} locale directory exists but is not in the web-ui i18n contract surface order`);
   }
   for (const locale of registryLocales.filter((item) => !supportedLocales.includes(item))) {
@@ -657,32 +552,28 @@ function auditSurfaceResourceRoots() {
       if (surface === 'web-ui') {
         const localeDir = path.join(resourceRoot, localeId);
         if (!fs.existsSync(localeDir)) {
           reportError(`${surface} is missing ${localeId} locale directory`);
         }
       } else if (surface === 'installer') {
         const installerLocale = localeById.get(localeId)?.installer?.uiCode;
         if (!installerLocale || !fs.existsSync(path.join(resourceRoot, `${installerLocale}.json`))) {
           reportError(`${surface} is missing ${localeId} resource JSON`);
         }
-      } else if (surface === 'core') {
-        if (!fs.existsSync(path.join(resourceRoot, `${localeId}.ftl`))) {
-          reportError(`${surface} is missing ${localeId} Fluent resource`);
-        }
-      } else if (surface === 'mobile-web') {
-        if (!fs.existsSync(path.join(resourceRoot, 'messages.ts'))) {
-          reportError(`${surface} is missing messages.ts`);
-        }
-      }
-    }
-  }
-}
+      } else if (surface === 'core') {
+        if (!fs.existsSync(path.join(resourceRoot, `${localeId}.ftl`))) {
+          reportError(`${surface} is missing ${localeId} Fluent resource`);
+        }
+      }
+    }
+  }
+}
 
 function auditGeneratedContract() {
   try {
     execFileSync(process.execPath, ['scripts/generate-i18n-contract.mjs', '--check'], {
       cwd: root,
       stdio: 'pipe',
     });
   } catch (error) {
     const stderr = error.stderr?.toString?.().trim();
     reportError(`Generated i18n contract files are out of date. Run pnpm run i18n:generate.${stderr ? ` ${stderr}` : ''}`);
@@ -716,46 +607,27 @@ function auditSharedTermsCoverage() {
     try {
       keys = flattenKeys(readJsonFile(termsPath));
     } catch (error) {
       reportError(`Failed to parse ${toPosixPath(path.relative(root, termsPath))}: ${error.message}`);
       continue;
     }
 
     for (const key of diffSets(baselineKeys, keys)) {
       reportError(`${localeId} shared terms.json is missing key "${key}"`);
     }
-    for (const key of diffSets(keys, baselineKeys)) {
-      reportError(`${localeId} shared terms.json has extra key "${key}"`);
-    }
-  }
-}
-
-function auditMobileWebBoundary() {
-  const sourceFiles = listFiles(
-    mobileWebSourceDir,
-    (file) => file.endsWith('.ts') || file.endsWith('.tsx'),
-  );
-  const forbiddenPatterns = [
-    /src[/\\]web-ui[/\\]src[/\\]locales/,
-    /src[/\\]web-ui[/\\]src[/\\]infrastructure[/\\]i18n/,
-    /\.\.[/\\]\.\.[/\\]web-ui[/\\]/,
-  ];
-
-  for (const file of sourceFiles) {
-    const text = fs.readFileSync(file, 'utf8');
-    if (forbiddenPatterns.some((pattern) => pattern.test(text))) {
-      reportError(`${toPosixPath(path.relative(root, file))} imports or references web-ui i18n resources`);
-    }
-  }
-}
-
-function auditKeyParity(namespaces) {
+    for (const key of diffSets(keys, baselineKeys)) {
+      reportError(`${localeId} shared terms.json has extra key "${key}"`);
+    }
+  }
+}
+
+function auditKeyParity(namespaces) {
   for (const namespace of namespaces) {
     const baselineKeys = readJsonKeys(baselineLocale, namespace);
     for (const locale of supportedLocales.filter((item) => item !== baselineLocale)) {
       const localeKeys = readJsonKeys(locale, namespace);
       const missing = diffSets(baselineKeys, localeKeys);
       const extra = diffSets(localeKeys, baselineKeys);
 
       if (missing.length > 0) {
         reportError(`${locale}/${namespace}.json is missing ${missing.length} key(s): ${missing.slice(0, 8).join(', ')}`);
       }
@@ -773,75 +645,28 @@ function auditWebI18nextPlaceholderParity(namespaces) {
       Array.from(baselineEntries.entries()).map(([key, value]) => [
         key,
         extractI18nextPlaceholders(value),
       ]),
     );
 
     for (const locale of supportedLocales.filter((item) => item !== baselineLocale)) {
       const localeEntries = readJsonEntries(locale, namespace);
       for (const [key, expected] of baselinePlaceholders.entries()) {
         if (!localeEntries.has(key)) continue;
-        const actual = extractI18nextPlaceholders(localeEntries.get(key));
-        reportPlaceholderParity(`web-ui ${namespace}`, locale, key, expected, actual);
-      }
-    }
-  }
-}
-
-function auditMobileWebMessageParity() {
-  const messagesByLocale = readMobileMessageKeysByLocale();
-  const baselineKeys = messagesByLocale.get('en-US');
-  if (!baselineKeys) {
-    reportError('mobile-web messages are missing the en-US baseline locale');
-    return;
-  }
-
-  for (const [locale, keys] of messagesByLocale.entries()) {
-    if (locale === 'en-US') continue;
-
-    const missing = diffSets(baselineKeys, keys);
-    const extra = diffSets(keys, baselineKeys);
-    if (missing.length > 0) {
-      reportError(`mobile-web ${locale} messages are missing ${missing.length} key(s): ${missing.slice(0, 8).join(', ')}`);
-    }
-    if (extra.length > 0) {
-      reportError(`mobile-web ${locale} messages have ${extra.length} extra key(s): ${extra.slice(0, 8).join(', ')}`);
-    }
-  }
-}
-
-function auditMobileWebPlaceholderParity() {
-  const messagesByLocale = readMobileMessagesByLocale();
-  const baselineEntries = messagesByLocale.get('en-US');
-  if (!baselineEntries) {
-    reportError('mobile-web messages are missing the en-US baseline locale');
-    return;
-  }
-
-  const baselinePlaceholders = new Map(
-    Array.from(baselineEntries.entries()).map(([key, value]) => [
-      key,
-      extractMobilePlaceholders(value),
-    ]),
-  );
-
-  for (const [locale, entries] of messagesByLocale.entries()) {
-    if (locale === 'en-US') continue;
-    for (const [key, expected] of baselinePlaceholders.entries()) {
-      if (!entries.has(key)) continue;
-      const actual = extractMobilePlaceholders(entries.get(key));
-      reportPlaceholderParity('mobile-web', locale, key, expected, actual);
-    }
-  }
-}
-
-function auditInstallerKeyParity() {
+        const actual = extractI18nextPlaceholders(localeEntries.get(key));
+        reportPlaceholderParity(`web-ui ${namespace}`, locale, key, expected, actual);
+      }
+    }
+  }
+}
+
+function auditInstallerKeyParity() {
   const baselineKeys = readInstallerJsonKeys('en');
   for (const uiLocale of ['zh', 'zh-TW']) {
     const keys = readInstallerJsonKeys(uiLocale);
     const missing = diffSets(baselineKeys, keys);
     const extra = diffSets(keys, baselineKeys);
 
     if (missing.length > 0) {
       reportError(`installer ${uiLocale}.json is missing ${missing.length} key(s): ${missing.slice(0, 8).join(', ')}`);
     }
     if (extra.length > 0) {
@@ -960,41 +785,27 @@ function collectI18nResourceEntries(namespaces) {
       const file = namespace === 'shared'
         ? `src/shared/i18n/resources/shared/${locale}/terms.json`
         : `src/web-ui/src/locales/${locale}/${namespace}.json`;
       for (const [key, value] of readJsonEntries(locale, namespace)) {
         pushResourceEntry(entries, {
           surface,
           locale,
           namespace: namespace === 'shared' ? 'shared' : namespace,
           key,
           value,
-          file,
-        });
-      }
-    }
-  }
-
-  if (auditTypeScript) {
-    for (const [locale, messageEntries] of readMobileMessagesByLocale().entries()) {
-      for (const [key, value] of messageEntries.entries()) {
-        pushResourceEntry(entries, {
-          surface: 'mobile-web',
-          locale,
-          key,
-          value,
-          file: 'src/mobile-web/src/i18n/messages.ts',
-        });
-      }
-    }
-  }
-
-  for (const localeId of localeContract.surfaceOrders?.installer ?? []) {
+          file,
+        });
+      }
+    }
+  }
+
+  for (const localeId of localeContract.surfaceOrders?.installer ?? []) {
     const uiLocale = localeById.get(localeId)?.installer?.uiCode;
     if (!uiLocale) continue;
     for (const [key, value] of readInstallerJsonEntries(uiLocale).entries()) {
       pushResourceEntry(entries, {
         surface: 'installer',
         locale: localeId,
         key,
         value,
         file: `northhing-Installer/src/i18n/locales/${uiLocale}.json`,
       });
@@ -1571,37 +1382,25 @@ function auditI18nGovernanceReport(namespaces) {
 
 function shouldSkipSourceScan(file) {
   const normalized = toPosixPath(path.relative(root, file));
   return (
     normalized.includes('/locales/') ||
     normalized.endsWith('/generatedLocaleContract.ts') ||
     normalized.endsWith('.test.ts') ||
     normalized.endsWith('.test.tsx') ||
     normalized.endsWith('.spec.ts') ||
     normalized.endsWith('.spec.tsx') ||
-    normalized.includes('/component-library/components/registry.tsx')
-  );
-}
-
-function shouldSkipMobileWebSourceScan(file) {
-  const normalized = toPosixPath(path.relative(root, file));
-  return (
-    normalized.endsWith('/i18n/messages.ts') ||
-    normalized.endsWith('/i18n/generatedLocaleContract.ts') ||
-    normalized.endsWith('.test.ts') ||
-    normalized.endsWith('.test.tsx') ||
-    normalized.endsWith('.spec.ts') ||
-    normalized.endsWith('.spec.tsx')
-  );
-}
-
-function shouldSkipInstallerSourceScan(file) {
+    normalized.includes('/component-library/components/registry.tsx')
+  );
+}
+
+function shouldSkipInstallerSourceScan(file) {
   const normalized = toPosixPath(path.relative(root, file));
   return (
     normalized.includes('/i18n/locales/') ||
     normalized.endsWith('/i18n/generatedLocaleContract.ts') ||
     normalized.endsWith('.test.ts') ||
     normalized.endsWith('.test.tsx') ||
     normalized.endsWith('.spec.ts') ||
     normalized.endsWith('.spec.tsx')
   );
 }
@@ -1984,71 +1783,61 @@ function countCjkSourceLines(scanRoot, predicate) {
     lines.forEach((line, index) => {
       if (cjkPattern.test(line)) {
         findings.push(`${toPosixPath(path.relative(root, file))}:${index + 1}`);
       }
     });
   }
 
   return findings;
 }
 
-function shouldSkipLocaleFormatSourceScan(file) {
-  const normalized = toPosixPath(path.relative(root, file));
-  return (
-    // Surface i18n owners are the only approved locations for direct Intl usage;
-    // product code must call their exported formatting helpers instead.
-    normalized === 'src/web-ui/src/infrastructure/i18n/core/I18nService.ts' ||
-    normalized === 'src/mobile-web/src/i18n/I18nProvider.tsx' ||
-    normalized.endsWith('/generatedLocaleContract.ts') ||
+function shouldSkipLocaleFormatSourceScan(file) {
+  const normalized = toPosixPath(path.relative(root, file));
+  return (
+    // Surface i18n owners are the only approved locations for direct Intl usage;
+    // product code must call their exported formatting helpers instead.
+    normalized === 'src/web-ui/src/infrastructure/i18n/core/I18nService.ts' ||
+    normalized.endsWith('/generatedLocaleContract.ts') ||
     normalized.endsWith('.test.ts') ||
     normalized.endsWith('.test.tsx') ||
     normalized.endsWith('.spec.ts') ||
     normalized.endsWith('.spec.tsx')
   );
 }
 
-function createLocaleFormatScanSpecs() {
-  return [
-    {
-      surface: 'web-ui',
-      root: webSourceDir,
-      predicate: (file) => (
-        (file.endsWith('.ts') || file.endsWith('.tsx')) &&
-        !shouldSkipSourceScan(file) &&
-        !shouldSkipLocaleFormatSourceScan(file)
-      ),
-    },
-    {
-      surface: 'mobile-web',
-      root: mobileWebSourceDir,
-      predicate: (file) => (
-        (file.endsWith('.ts') || file.endsWith('.tsx')) &&
-        !shouldSkipMobileWebSourceScan(file) &&
-        !shouldSkipLocaleFormatSourceScan(file)
-      ),
-    },
-    {
-      surface: 'installer',
-      root: installerSourceDir,
-      predicate: (file) => (
-        (file.endsWith('.ts') || file.endsWith('.tsx')) &&
-        !shouldSkipInstallerSourceScan(file) &&
-        !shouldSkipLocaleFormatSourceScan(file)
-      ),
-    },
-    {
-      surface: 'core-miniapp',
-      root: path.join(root, 'src', 'crates', 'contracts', 'product-domains', 'src', 'miniapp', 'builtin', 'assets'),
-      predicate: (file) => file.endsWith('.js'),
-    },
-  ];
-}
+function createLocaleFormatScanSpecs() {
+  return [
+    {
+      surface: 'web-ui',
+      root: webSourceDir,
+      predicate: (file) => (
+        (file.endsWith('.ts') || file.endsWith('.tsx')) &&
+        !shouldSkipSourceScan(file) &&
+        !shouldSkipLocaleFormatSourceScan(file)
+      ),
+    },
+    {
+      surface: 'installer',
+      root: installerSourceDir,
+      predicate: (file) => (
+        (file.endsWith('.ts') || file.endsWith('.tsx')) &&
+        !shouldSkipInstallerSourceScan(file) &&
+        !shouldSkipLocaleFormatSourceScan(file)
+      ),
+    },
+    {
+      surface: 'core-miniapp',
+      root: path.join(root, 'src', 'crates', 'contracts', 'product-domains', 'src', 'miniapp', 'builtin', 'assets'),
+      predicate: (file) => file.endsWith('.js'),
+    },
+  ];
+}
 
 function collectLocaleFormatSurfaceIds() {
   return sortedUnique(createLocaleFormatScanSpecs().map((spec) => spec.surface));
 }
 
 function collectLocaleFormatUsageFindings() {
   const formatPattern = /\b(?:new\s+)?Intl\.(?:DateTimeFormat|NumberFormat|RelativeTimeFormat)\s*\(|\.\s*toLocale(?:String|DateString|TimeString)\s*\(/g;
   const specs = createLocaleFormatScanSpecs();
   const findings = [];
 
@@ -2124,69 +1913,61 @@ function auditLocaleFormatUsageBudget() {
       reportError(`${file} no longer has direct locale formatting call(s); remove it from scripts/i18n-locale-format-baseline.json.`);
     }
   }
 }
 
 function auditHardcodedSourceBudgets() {
   const baseline = readJsonFile(hardcodedBaselinePath);
   const budgetById = new Map((baseline.budgets ?? []).map((budget) => [budget.id, budget.maxCjkLines]));
   // Baselines are a no-new-hardcoded-copy gate. Lower them as strings move to
   // owned locale resources; do not raise them for new user-facing text.
-  const specs = [
-    {
-      id: 'web-ui-source',
-      root: webSourceDir,
-      predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipSourceScan(file),
-    },
-    {
-      id: 'mobile-web-source',
-      root: mobileWebSourceDir,
-      predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipMobileWebSourceScan(file),
-    },
-    {
-      id: 'installer-source',
-      root: installerSourceDir,
-      predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipInstallerSourceScan(file),
-    },
-  ];
+  const specs = [
+    {
+      id: 'web-ui-source',
+      root: webSourceDir,
+      predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipSourceScan(file),
+    },
+    {
+      id: 'installer-source',
+      root: installerSourceDir,
+      predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipInstallerSourceScan(file),
+    },
+  ];
 
   for (const spec of specs) {
     const maxCjkLines = budgetById.get(spec.id);
     if (typeof maxCjkLines !== 'number') {
       reportError(`Missing hardcoded CJK budget for ${spec.id}`);
       continue;
     }
 
     const findings = countCjkSourceLines(spec.root, spec.predicate);
     if (findings.length > maxCjkLines) {
       reportError(`${spec.id} has ${findings.length} CJK source candidate line(s), budget is ${maxCjkLines}. First entries: ${findings.slice(0, 12).join(', ')}`);
     } else if (findings.length > 0) {
       reportWarning(`${spec.id} has ${findings.length} grandfathered CJK source candidate line(s). First entries: ${findings.slice(0, 12).join(', ')}`);
     }
   }
 }
 
-auditGeneratedContract();
-auditSharedTermsCoverage();
-auditSurfaceResourceRoots();
-auditMobileWebBoundary();
-
-const namespaces = auditNamespaceCoverage();
-auditKeyParity(namespaces);
-auditWebI18nextPlaceholderParity(namespaces);
-auditTypeScript = loadTypeScriptForAudit();
-if (auditTypeScript) {
-  auditWebUiStaticTranslationKeys(namespaces);
-  auditWebUiLiteralFallbackBudget();
-  auditMobileWebMessageParity();
-  auditMobileWebPlaceholderParity();
-}
+auditGeneratedContract();
+auditSharedTermsCoverage();
+auditSurfaceResourceRoots();
+
+const namespaces = auditNamespaceCoverage();
+auditKeyParity(namespaces);
+auditWebI18nextPlaceholderParity(namespaces);
+auditTypeScript = loadTypeScriptForAudit();
+if (auditTypeScript) {
+  auditWebUiStaticTranslationKeys(namespaces);
+  auditWebUiLiteralFallbackBudget();
+}
 auditInstallerKeyParity();
 auditInstallerPlaceholderParity();
 auditCoreFluentParity();
 auditSourceText();
 auditLocaleFormatUsageBudget();
 auditHardcodedSourceBudgets();
 auditI18nGovernanceReport(namespaces);
 writeGovernanceReport();
 
 if (errorCount > 0) {
diff --git a/scripts/i18n-contract.test.mjs b/scripts/i18n-contract.test.mjs
index e983bdd..a654515 100644
--- a/scripts/i18n-contract.test.mjs
+++ b/scripts/i18n-contract.test.mjs
@@ -5,21 +5,20 @@ import path from 'node:path';
 import test from 'node:test';
 
 const root = process.cwd();
 const contractTestProfile = process.env.northhing_I18N_CONTRACT_TEST_PROFILE ?? 'full';
 const runAuditIntegrationTests = process.env.northhing_I18N_CONTRACT_TEST_AUDIT_INTEGRATION === '1';
 const skipAuditIntegrationTests = contractTestProfile === 'ci' && !runAuditIntegrationTests;
 const contractPath = path.join(root, 'src', 'shared', 'i18n', 'contract', 'locales.json');
 const sharedTermsDir = path.join(root, 'src', 'shared', 'i18n', 'resources', 'shared');
 const expectedGeneratedFiles = [
   'src/web-ui/src/infrastructure/i18n/presets/generatedLocaleContract.ts',
-  'src/mobile-web/src/i18n/generatedLocaleContract.ts',
   'northhing-Installer/src/i18n/generatedLocaleContract.ts',
   'src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs',
   'northhing-Installer/src-tauri/src/installer/generated_locale_contract.rs',
 ];
 const expectedGeneratedJsonFiles = [];
 
 function readJson(relativePath) {
   return JSON.parse(fs.readFileSync(path.join(root, relativePath), 'utf8'));
 }
 
@@ -169,49 +168,38 @@ test('core runtime uses the generated locale contract for language identity', ()
     resourceRegistrySource,
     /\b(name|english_name|native_name|rtl|model_language_name|short_model_instruction|aliases):/,
     'backend locale_registry.rs should only own Fluent resource wiring, not duplicate locale identity',
   );
 });
 
 test('shared i18n terms are consumed by each product surface runtime', () => {
   const webI18nSource = readText('src/web-ui/src/infrastructure/i18n/core/I18nService.ts');
   assert.match(webI18nSource, /SHARED_TERMS_BY_LOCALE/, 'Web UI should merge shared terms into i18next resources');
 
-  const mobileMessagesSource = readText('src/mobile-web/src/i18n/messages.ts');
-  assert.match(mobileMessagesSource, /SHARED_TERMS_BY_LOCALE/, 'mobile-web should expose shared terms through its message tree');
-
   const installerLanguagesSource = readText('northhing-Installer/src/i18n/languages.ts');
   assert.match(installerLanguagesSource, /SHARED_TERMS_BY_APP_LANGUAGE/, 'installer should merge shared terms into its i18next resources');
 
   const coreServiceSource = readText('src/crates/assembly/core/src/service/i18n/service.rs');
   assert.match(coreServiceSource, /generated_shared_term/, 'core i18n service should resolve generated shared terms');
 });
 
 test('frontend runtimes use generated locale defaults and fallback chains', () => {
   const webPresetIndexSource = readText('src/web-ui/src/infrastructure/i18n/presets/index.ts');
   assert.doesNotMatch(
     webPresetIndexSource,
     /export const DEFAULT_(?:FALLBACK_)?LOCALE\s*=\s*['"]/,
     'Web UI preset defaults should be re-exported from the generated contract, not hard-coded',
   );
 
   const webI18nSource = readText('src/web-ui/src/infrastructure/i18n/core/I18nService.ts');
   assert.match(webI18nSource, /getLocaleFallbackChain/, 'Web UI i18next should use the generated locale fallback chain');
 
-  const mobileProviderSource = readText('src/mobile-web/src/i18n/I18nProvider.tsx');
-  assert.match(mobileProviderSource, /getMobileFallbackChain/, 'mobile-web translate should use the generated locale fallback chain');
-  assert.doesNotMatch(
-    mobileProviderSource,
-    /messages\[DEFAULT_LANGUAGE\]/,
-    'mobile-web translate should not fall back directly to the surface default only',
-  );
-
   const installerI18nSource = readText('northhing-Installer/src/i18n/index.ts');
   assert.match(installerI18nSource, /DEFAULT_INSTALLER_UI_LANGUAGE/, 'installer i18next should use the generated default UI language');
   assert.match(installerI18nSource, /getInstallerUiFallbackChain/, 'installer i18next should use the generated locale fallback chain');
 });
 
 test('web-ui runtime keeps locale namespaces lazy outside its bootstrap set', () => {
   const webI18nSource = readText('src/web-ui/src/infrastructure/i18n/core/I18nService.ts');
 
   assert.match(webI18nSource, /WEB_UI_BOOTSTRAP_NAMESPACES/, 'Web UI should declare the small eager namespace set');
   assert.match(webI18nSource, /import\.meta\.glob\('\.\.\/\.\.\/\.\.\/locales\/\*\*\/\*\.json'/, 'Web UI should keep a lazy namespace glob for non-bootstrap resources');
@@ -327,46 +315,43 @@ test('i18n audit enforces the checked-in hardcoded source candidate budget', ()
   assert.match(auditSource, /i18n-hardcoded-baseline\.json/, 'i18n:audit should read the hardcoded copy baseline');
   assert.match(auditSource, /auditHardcodedSourceBudgets/, 'i18n:audit should fail when hardcoded candidate budgets grow');
 });
 
 test('i18n audit treats locale key parity as an error', () => {
   const auditSource = readText('scripts/i18n-audit.mjs');
   const parityFunction = auditSource.match(/function auditKeyParity\(namespaces\) \{[\s\S]*?\n\}/)?.[0] ?? '';
 
   assert.match(parityFunction, /reportError\(`\$\{locale\}\/\$\{namespace\}\.json is missing/, 'missing locale keys should fail i18n:audit');
   assert.match(parityFunction, /reportError\(`\$\{locale\}\/\$\{namespace\}\.json has/, 'extra locale keys should fail i18n:audit');
-  assert.match(auditSource, /auditMobileWebMessageParity/, 'mobile-web message keys should be covered by i18n:audit');
   assert.match(auditSource, /auditInstallerKeyParity/, 'installer locale keys should be covered by i18n:audit');
 });
 
 test('CI runs i18n contract and audit guards before frontend builds', () => {
   const ciSource = readText('.github/workflows/ci.yml');
   const contractIndex = ciSource.indexOf('pnpm run i18n:contract:test:ci');
   const auditIndex = ciSource.indexOf('pnpm run i18n:audit');
   const buildIndex = ciSource.indexOf('pnpm run build:web');
 
   assert.notEqual(contractIndex, -1, 'CI should run pnpm run i18n:contract:test:ci');
   assert.notEqual(auditIndex, -1, 'CI should run pnpm run i18n:audit');
   assert.ok(contractIndex < buildIndex, 'i18n contract checks should run before web build');
   assert.ok(contractIndex < auditIndex, 'CI should run the fast contract check before the full i18n audit');
   assert.ok(auditIndex < buildIndex, 'i18n audit should run before web build');
 });
 
 test('i18n audit enforces interpolation parameter parity across resource formats', () => {
   const auditSource = readText('scripts/i18n-audit.mjs');
 
   assert.match(auditSource, /auditWebI18nextPlaceholderParity/, 'Web UI JSON placeholders should be audited');
-  assert.match(auditSource, /auditMobileWebPlaceholderParity/, 'mobile-web placeholders should be audited');
   assert.match(auditSource, /auditInstallerPlaceholderParity/, 'installer placeholders should be audited');
   assert.match(auditSource, /auditCoreFluentParity/, 'core Fluent keys and placeholders should be audited');
   assert.match(auditSource, /extractI18nextPlaceholders/, 'i18next placeholder extraction should be explicit');
-  assert.match(auditSource, /extractMobilePlaceholders/, 'mobile placeholder extraction should be explicit');
   assert.match(auditSource, /extractFluentPlaceholders/, 'Fluent placeholder extraction should be explicit');
 });
 
 test('i18n audit report surface summaries derive from owned scan and budget sources', () => {
   const auditSource = readText('scripts/i18n-audit.mjs');
 
   assert.doesNotMatch(
     auditSource,
     /const governanceSurfaceIds\s*=\s*\[/,
     'governance report surface summaries should derive from the governance baseline dimensions',
@@ -620,92 +605,20 @@ auditIntegrationTest('i18n audit reports same-text zh-TW copy with a l10n signal
           entry.allowlistState === 'unreviewed'
         )),
         'audit inventory should retain the same-text pair even though only signal-bearing entries are governance candidates',
       );
     });
   } finally {
     fs.rmSync(absoluteReportPath, { force: true });
   }
 });
 
-auditIntegrationTest('mobile-web uses shared terms for stable shared concept labels', { concurrency: false }, () => {
-  const reportPath = 'scripts/.tmp-i18n-mobile-shared-terms-report.json';
-  const absoluteReportPath = path.join(root, reportPath);
-  fs.rmSync(absoluteReportPath, { force: true });
-
-  try {
-    const result = runI18nAudit(['--report-json', reportPath]);
-    assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
-
-    const report = readJson(reportPath);
-    const migratedSharedKeys = new Set([
-      'product.remote',
-      'features.workspace',
-      'modes.assistant',
-      'modes.expert',
-      'agents.claw',
-      'agents.code',
-      'agents.cowork',
-      'agents.default',
-      'tools.edit',
-      'tools.explore',
-      'tools.read',
-      'tools.shell',
-      'tools.todo',
-      'tools.write',
-    ]);
-    const mobileDuplicates = report.sharedTermDuplicates
-      .filter((entry) => entry.surface === 'mobile-web' && migratedSharedKeys.has(entry.sharedKey))
-      .map((entry) => `${entry.sharedKey}:${entry.key}:${entry.locale}`)
-      .sort();
-    const legacyMobileKeys = [
-      'common.appName',
-      'sessions.workspace',
-      'sessions.assistantMode',
-      'sessions.coworkSession',
-      'sessions.codeSession',
-      'sessions.defaultAssistant',
-      'sessions.agentClaw',
-      'sessions.proMode',
-      'workspace.title',
-      'tools.edit',
-      'tools.explore',
-      'tools.read',
-      'tools.shell',
-      'tools.todo',
-      'tools.write',
-    ];
-    const mobileSourceFiles = listFiles(path.join(root, 'src', 'mobile-web', 'src'), (file) => (
-      /\.(?:ts|tsx)$/.test(file) && !file.endsWith(`${path.sep}i18n${path.sep}messages.ts`)
-    ));
-    const legacyReferences = mobileSourceFiles.flatMap((file) => {
-      const source = fs.readFileSync(file, 'utf8');
-      return legacyMobileKeys
-        .filter((key) => source.includes(`'${key}'`) || source.includes(`"${key}"`))
-        .map((key) => `${path.relative(root, file)}:${key}`);
-    }).sort();
-
-    assert.deepEqual(
-      mobileDuplicates,
-      [],
-      'mobile-web should read migrated stable labels from shared terms instead of copying values',
-    );
-    assert.deepEqual(
-      legacyReferences,
-      [],
-      'mobile-web source should not call removed local keys for migrated shared terms',
-    );
-  } finally {
-    fs.rmSync(absoluteReportPath, { force: true });
-  }
-});
-
 auditIntegrationTest('web-ui uses shared terms for stable navigation and feature labels', { concurrency: false }, () => {
   const reportPath = 'scripts/.tmp-i18n-web-ui-shared-terms-report.json';
   const absoluteReportPath = path.join(root, reportPath);
   fs.rmSync(absoluteReportPath, { force: true });
 
   try {
     const result = runI18nAudit(['--report-json', reportPath]);
     assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
 
     const report = readJson(reportPath);
@@ -1116,16 +1029,14 @@ test('i18n contract surface resource roots point at existing owned resources', (
 
     for (const localeId of contract.surfaceOrders[surface] ?? []) {
       if (surface === 'web-ui') {
         assert.ok(fs.existsSync(path.join(resourceRoot, localeId)), `${surface} is missing ${localeId} locale directory`);
       } else if (surface === 'installer') {
         const locale = localeById.get(localeId);
         assert.ok(locale?.installer?.uiCode, `${localeId} is missing installer.uiCode`);
         assert.ok(fs.existsSync(path.join(resourceRoot, `${locale.installer.uiCode}.json`)), `${surface} is missing ${localeId} resource JSON`);
       } else if (surface === 'core') {
         assert.ok(fs.existsSync(path.join(resourceRoot, `${localeId}.ftl`)), `${surface} is missing ${localeId} Fluent resource`);
-      } else if (surface === 'mobile-web') {
-        assert.ok(fs.existsSync(path.join(resourceRoot, 'messages.ts')), `${surface} is missing messages.ts`);
       }
     }
   }
 });
diff --git a/scripts/i18n-governance-baseline.json b/scripts/i18n-governance-baseline.json
index 5c59070..6ed27c1 100644
--- a/scripts/i18n-governance-baseline.json
+++ b/scripts/i18n-governance-baseline.json
@@ -3,21 +3,20 @@
   "description": "No-growth baseline for i18n governance candidates. Lower counts when shared-term or l10n debt is removed; do not raise without review.",
   "budgets": {
     "confirmedUnusedKeys": {
       "maxTotal": 0
     },
     "sharedTermDuplicates": {
       "maxTotal": 185,
       "bySurface": {
         "core": 15,
         "installer": 0,
-        "mobile-web": 0,
         "web-ui": 170
       },
       "bySharedKey": {
         "agents.claw": 3,
         "agents.code": 0,
         "agents.cowork": 0,
         "agents.default": 2,
         "connectionMethods.northhingServer": 0,
         "connectionMethods.lan": 0,
         "features.codeAgent": 1,
@@ -38,16 +37,15 @@
         "tools.explore": 2,
         "tools.search": 12,
         "tools.shell": 7
       }
     },
     "l10nQualityCandidates": {
       "maxTotal": 0,
       "bySurface": {
         "core": 0,
         "installer": 0,
-        "mobile-web": 0,
         "web-ui": 0
       }
     }
   }
 }
diff --git a/scripts/i18n-hardcoded-baseline.json b/scripts/i18n-hardcoded-baseline.json
index b9b658f..e28d261 100644
--- a/scripts/i18n-hardcoded-baseline.json
+++ b/scripts/i18n-hardcoded-baseline.json
@@ -1,17 +1,13 @@
 {
   "version": 1,
   "budgets": [
     {
       "id": "web-ui-source",
       "maxCjkLines": 0
     },
-    {
-      "id": "mobile-web-source",
-      "maxCjkLines": 0
-    },
     {
       "id": "installer-source",
       "maxCjkLines": 0
     }
   ]
 }
diff --git a/src/shared/i18n/contract/locales.json b/src/shared/i18n/contract/locales.json
index ca9936b..a4f646e 100644
--- a/src/shared/i18n/contract/locales.json
+++ b/src/shared/i18n/contract/locales.json
@@ -1,55 +1,45 @@
 {
   "version": 1,
   "defaultLocale": "zh-CN",
   "fallbackLocale": "en-US",
   "unknownLocaleFallbackChain": [
     "en-US",
     "zh-CN"
   ],
   "surfaceDefaults": {
     "web-ui": "zh-CN",
-    "mobile-web": "en-US",
     "installer": "en-US",
     "core": "zh-CN"
   },
   "surfaceOrders": {
     "web-ui": [
       "zh-CN",
       "en-US",
       "zh-TW"
     ],
-    "mobile-web": [
-      "zh-CN",
-      "zh-TW",
-      "en-US"
-    ],
     "installer": [
       "en-US",
       "zh-CN",
       "zh-TW"
     ],
     "core": [
       "zh-CN",
       "zh-TW",
       "en-US"
     ]
   },
   "surfaces": {
     "web-ui": {
       "resourceRoot": "src/web-ui/src/locales",
       "loading": "eager-namespaces"
     },
-    "mobile-web": {
-      "resourceRoot": "src/mobile-web/src/i18n",
-      "loading": "surface-minimal"
-    },
     "installer": {
       "resourceRoot": "northhing-Installer/src/i18n/locales",
       "loading": "surface-minimal"
     },
     "core": {
       "resourceRoot": "src/crates/assembly/core/locales",
       "loading": "backend-service"
     }
   },
   "locales": [
