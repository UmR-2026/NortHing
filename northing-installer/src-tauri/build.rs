use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(mobile)");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let locales_dir = Path::new(&manifest_dir)
        .join("..")
        .join("src")
        .join("i18n")
        .join("locales");

    let locale_codes = discover_locale_codes(&locales_dir);
    let contract = generate_locale_contract(&locale_codes);

    let out_path = Path::new(&manifest_dir)
        .join("src")
        .join("installer")
        .join("generated_locale_contract.rs");
    fs::write(&out_path, contract).expect("failed to write generated_locale_contract.rs");

    println!("cargo:rerun-if-changed={}", locales_dir.display());
    println!("cargo:rerun-if-changed=build.rs");
}

fn discover_locale_codes(locales_dir: &Path) -> Vec<String> {
    let mut codes = Vec::new();
    if let Ok(entries) = fs::read_dir(locales_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    codes.push(stem.to_string());
                }
            }
        }
    }
    codes.sort();
    codes
}

fn generate_locale_contract(codes: &[String]) -> String {
    let mut lang_defs: Vec<(String, String, Vec<String>)> = Vec::new();
    for code in codes {
        let (variant, app_code, aliases) = match code.as_str() {
            "en" => (
                "En".to_string(),
                "en-US".to_string(),
                vec!["en".to_string(), "en-US".to_string()],
            ),
            "zh" => (
                "Zh".to_string(),
                "zh-CN".to_string(),
                vec!["zh".to_string(), "zh-Hans".to_string(), "zh-CN".to_string()],
            ),
            "zh-TW" => (
                "ZhTw".to_string(),
                "zh-TW".to_string(),
                vec![
                    "zh-TW".to_string(),
                    "zh-Hant".to_string(),
                    "zh-HK".to_string(),
                    "zh-MO".to_string(),
                ],
            ),
            other => (
                other_to_variant(other),
                other.to_string(),
                vec![other.to_string()],
            ),
        };
        lang_defs.push((variant, app_code, aliases));
    }

    let mut out = String::new();
    out.push_str("// Generated from src/i18n/locales/*.json by build.rs.\n");
    out.push_str("// Do not edit by hand; edit the locale JSON files and rebuild.\n\n");

    // InstallerUiLanguage enum
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    out.push_str("pub enum InstallerUiLanguage {\n");
    for (variant, _, _) in &lang_defs {
        out.push_str(&format!("    {},\n", variant));
    }
    out.push_str("}\n\n");

    out.push_str("impl InstallerUiLanguage {\n");
    out.push_str("    pub fn as_str(&self) -> &'static str {\n");
    out.push_str("        match self {\n");
    for (variant, _, aliases) in &lang_defs {
        let primary = aliases.first().map(|s| s.as_str()).unwrap_or("en");
        out.push_str(&format!("            Self::{} => \"{}\",\n", variant, primary));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // AppLanguage enum
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    out.push_str("pub enum AppLanguage {\n");
    for (_, app_code, _) in &lang_defs {
        let app_variant = match app_code.as_str() {
            "en-US" => "EnUs".to_string(),
            "zh-CN" => "ZhCn".to_string(),
            "zh-TW" => "ZhTw".to_string(),
            other => other_to_variant(other),
        };
        out.push_str(&format!("    {},\n", app_variant));
    }
    out.push_str("}\n\n");

    out.push_str("impl AppLanguage {\n");
    out.push_str("    pub fn as_str(&self) -> &'static str {\n");
    out.push_str("        match self {\n");
    for (_, app_code, _) in &lang_defs {
        let app_variant = match app_code.as_str() {
            "en-US" => "EnUs".to_string(),
            "zh-CN" => "ZhCn".to_string(),
            "zh-TW" => "ZhTw".to_string(),
            other => other_to_variant(other),
        };
        out.push_str(&format!("            Self::{} => \"{}\",\n", app_variant, app_code));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // InstallerLanguageDefinition struct
    out.push_str("#[derive(Debug, Clone)]\n");
    out.push_str("pub struct InstallerLanguageDefinition {\n");
    out.push_str("    pub ui_code: InstallerUiLanguage,\n");
    out.push_str("    pub app_code: AppLanguage,\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    pub english_name: &'static str,\n");
    out.push_str("    pub native_name: &'static str,\n");
    out.push_str("    pub rtl: bool,\n");
    out.push_str("    pub aliases: &'static [&'static str],\n");
    out.push_str("    pub content_fallbacks: &'static [AppLanguage],\n");
    out.push_str("}\n\n");

    // Default language
    out.push_str("pub const DEFAULT_INSTALLER_UI_LANGUAGE: InstallerUiLanguage = InstallerUiLanguage::En;\n\n");

    // INSTALLER_LANGUAGE_DEFINITIONS
    out.push_str("pub const INSTALLER_LANGUAGE_DEFINITIONS: &[InstallerLanguageDefinition] = &[\n");
    for (i, (variant, app_code, aliases)) in lang_defs.iter().enumerate() {
        let app_variant = match app_code.as_str() {
            "en-US" => "EnUs".to_string(),
            "zh-CN" => "ZhCn".to_string(),
            "zh-TW" => "ZhTw".to_string(),
            other => other_to_variant(other),
        };
        let (name, english_name, native_name) = language_display_names(app_code);
        let aliases_str = aliases
            .iter()
            .map(|a| format!("\"{}\"", a))
            .collect::<Vec<_>>()
            .join(", ");
        let fallbacks = language_fallbacks(app_code);
        let fallbacks_str = fallbacks
            .iter()
            .map(|f| format!("AppLanguage::{}", f))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str("    InstallerLanguageDefinition {\n");
        out.push_str(&format!("        ui_code: InstallerUiLanguage::{},\n", variant));
        out.push_str(&format!("        app_code: AppLanguage::{},\n", app_variant));
        out.push_str(&format!("        name: \"{}\",\n", name));
        out.push_str(&format!("        english_name: \"{}\",\n", english_name));
        out.push_str(&format!("        native_name: \"{}\",\n", native_name));
        out.push_str("        rtl: false,\n");
        out.push_str(&format!("        aliases: &[{}],\n", aliases_str));
        out.push_str(&format!("        content_fallbacks: &[{}],\n", fallbacks_str));
        out.push_str("    }");
        if i + 1 < lang_defs.len() {
            out.push_str(",\n");
        } else {
            out.push_str("\n");
        }
    }
    out.push_str("];\n\n");

    // Helper functions
    out.push_str("pub fn installer_language_by_ui_code(code: InstallerUiLanguage) -> &'static InstallerLanguageDefinition {\n");
    out.push_str("    INSTALLER_LANGUAGE_DEFINITIONS\n");
    out.push_str("        .iter()\n");
    out.push_str("        .find(|def| def.ui_code == code)\n");
    out.push_str("        .expect(\"InstallerUiLanguage missing from INSTALLER_LANGUAGE_DEFINITIONS\")\n");
    out.push_str("}\n\n");

    out.push_str("pub fn shared_terms_by_app_language() -> std::collections::HashMap<AppLanguage, std::collections::HashMap<String, String>> {\n");
    out.push_str("    let mut map = std::collections::HashMap::new();\n");
    out.push_str("    let terms: &[(&str, &str)] = &[\n");
    out.push_str("        (\"product.name\", \"northhing\"),\n");
    out.push_str("        (\"product.remote\", \"northhing Remote\"),\n");
    out.push_str("    ];\n");
    out.push_str("    let languages = [\n");
    for (_, app_code, _) in &lang_defs {
        let app_variant = match app_code.as_str() {
            "en-US" => "EnUs".to_string(),
            "zh-CN" => "ZhCn".to_string(),
            "zh-TW" => "ZhTw".to_string(),
            other => other_to_variant(other),
        };
        out.push_str(&format!("        AppLanguage::{},\n", app_variant));
    }
    out.push_str("    ];\n");
    out.push_str("    for lang in languages {\n");
    out.push_str("        let entry = map.entry(lang).or_insert_with(std::collections::HashMap::new);\n");
    out.push_str("        for (key, value) in terms {\n");
    out.push_str("            entry.insert(key.to_string(), value.to_string());\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("    map\n");
    out.push_str("}\n");

    out
}

fn other_to_variant(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in s.chars() {
        if ch == '-' || ch == '_' {
            capitalize = true;
        } else if capitalize {
            out.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        out.push_str("Unknown");
    }
    out
}

fn language_display_names(code: &str) -> (&'static str, &'static str, &'static str) {
    match code {
        "en-US" => ("English", "English (US)", "English"),
        "zh-CN" => ("简体中文", "Simplified Chinese", "简体中文"),
        "zh-TW" => ("繁體中文", "Traditional Chinese", "繁體中文"),
        _ => ("Unknown", "Unknown", "Unknown"),
    }
}

fn language_fallbacks(code: &str) -> Vec<String> {
    match code {
        "en-US" => vec!["ZhCn".to_string()],
        "zh-CN" => vec!["EnUs".to_string()],
        "zh-TW" => vec!["ZhCn".to_string(), "EnUs".to_string()],
        _ => vec![],
    }
}
